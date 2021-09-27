#![warn(missing_docs)]
//! You can use [`run_app`] for [`App`]s created manually or generated from yaml and
//! [`run_derived`] for [`App`]s derived from a struct. Both of these functions take
//! a closure that contains the code that would normally be in `main`. They should be
//! the last thing you call in `main`.
//!
//! For example
//! ```no_run
//! # use clap::{App, Arg};
//! fn main() {
//!     let app = App::new("Example").arg(Arg::new("debug").short('d'));
//!     klask::run_app(app, |matches| {
//!        println!("{}", matches.is_present("debug"))
//!     });
//! }
//! ```
//! corresponds to
//! ```no_run
//! # use clap::{App, Arg};
//! fn main() {
//!     let app = App::new("Example").arg(Arg::new("debug").short('d'));
//!     let matches = app.get_matches();
//!     println!("{}", matches.is_present("debug"))
//! }
//! ```

mod app_state;
mod arg_state;
mod child_app;
mod error;
mod klask_ui;

use app_state::AppState;
use child_app::{ChildApp, StdinType};
use clap::{App, ArgMatches, FromArgMatches, IntoApp};
use eframe::{
    egui::{self, Button, Color32, CtxRef, Grid, Ui},
    epi,
};
use error::{ExecuteError, ValidationErrorInfo};
use klask_ui::KlaskUi;
use native_dialog::FileDialog;

/// Call with an [`App`] and a closure that contains the code that would normally be in `main`.
/// ```no_run
/// # use clap::{App, Arg};
/// let app = App::new("Example").arg(Arg::new("debug").short('d'));

/// klask::run_app(app, |matches| {
///    println!("{}", matches.is_present("debug"))
/// });
/// ```
pub fn run_app(app: App<'static>, f: impl FnOnce(&ArgMatches)) {
    // Wrap app in another in case no arguments is a valid configuration
    match App::new("outer").subcommand(app.clone()).try_get_matches() {
        Ok(matches) => match matches.subcommand_matches(app.get_name()) {
            // Called with arguments -> start user program
            Some(m) => f(m),
            // Called with no arguments -> start gui
            None => {
                let klask = Klask {
                    state: AppState::new(&app),
                    tab: Tab::Arguments,
                    env: Some(vec![]),
                    stdin: Some(StdinType::Text(String::new())),
                    working_dir: Some(String::new()),
                    output: None,
                    validation_error: None,
                    app,
                };
                let native_options = eframe::NativeOptions::default();
                eframe::run_native(Box::new(klask), native_options);
            }
        },
        Err(err) => panic!(
            "Internal error, arguments should've been empty or verified by the GUI app {:#?}",
            err
        ),
    }
}

/// Can be used with a struct deriving [`clap::Clap`]. Call with a closure that contains the code that would normally be in `main`.
/// It's just a wrapper over [`run_app`].
/// ```no_run
/// # use clap::{App, Arg, Clap};
/// #[derive(Clap)]
/// struct Example {
///     #[clap(short)]
///     debug: bool,
/// }
///
/// klask::run_derived::<Example, _>(|example|{
///     println!("{}", example.debug);
/// });
/// ```
pub fn run_derived<C, F>(f: F)
where
    C: IntoApp + FromArgMatches,
    F: FnOnce(C),
{
    run_app(C::into_app(), |m| {
        let matches = C::from_arg_matches(m)
            .expect("Internal error, C::from_arg_matches should always succeed");
        f(matches);
    });
}

#[derive(Debug)]
struct Klask {
    state: AppState,
    tab: Tab,
    env: Option<Vec<(String, String)>>,
    stdin: Option<StdinType>,
    working_dir: Option<String>,
    output: Option<Result<ChildApp, ExecuteError>>,
    validation_error: Option<ValidationErrorInfo>,
    // This isn't a generic lifetime because eframe::run_native() requires
    // a 'static lifetime because boxed trait objects default to 'static
    app: App<'static>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum Tab {
    Arguments,
    Env,
    Stdin,
}

impl epi::App for Klask {
    fn name(&self) -> &str {
        self.app.get_name()
    }

    fn update(&mut self, ctx: &CtxRef, _frame: &mut epi::Frame<'_>) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::auto_sized().show(ui, |ui| {
                let cols = 1
                    + if self.env.is_some() { 1 } else { 0 }
                    + if self.stdin.is_some() { 1 } else { 0 };

                if cols > 1 {
                    ui.columns(cols, |ui| {
                        let mut ui = ui.iter_mut();
                        ui.next().unwrap().selectable_value(
                            &mut self.tab,
                            Tab::Arguments,
                            "Arguments",
                        );
                        if self.env.is_some() {
                            ui.next().unwrap().selectable_value(
                                &mut self.tab,
                                Tab::Env,
                                "Environment variables",
                            );
                        }
                        if self.stdin.is_some() {
                            ui.next()
                                .unwrap()
                                .selectable_value(&mut self.tab, Tab::Stdin, "Input");
                        }
                    });
                    ui.separator();
                }

                match self.tab {
                    Tab::Arguments => {
                        self.state.update(ui, &mut self.validation_error);

                        if let Some(path) = &mut self.working_dir {
                            ui.horizontal(|ui| {
                                if ui.button("Select directory...").clicked() {
                                    if let Some(file) =
                                        FileDialog::new().show_open_single_dir().ok().flatten()
                                    {
                                        *path = file.to_string_lossy().into_owned();
                                    }
                                }
                                ui.text_edit_singleline_hint(path, "Working directory");
                            });
                            ui.add_space(10.0);
                        }
                    }
                    Tab::Env => self.update_env(ui),
                    Tab::Stdin => self.update_stdin(ui),
                }

                ui.horizontal(|ui| {
                    if ui
                        .add(Button::new("Run!").enabled(!self.is_child_running()))
                        .clicked()
                    {
                        self.output = Some(self.execute());
                        self.validation_error =
                            if let Some(Err(ExecuteError::ValidationError(info))) = &self.output {
                                Some(info.clone())
                            } else {
                                None
                            };
                    }

                    if self.is_child_running() {
                        if ui.button("Kill").clicked() {
                            self.kill_child();
                        }

                        let mut running_text = String::from("Running");
                        for _ in 0..((2.0 * ui.input().time) as i32 % 4) {
                            running_text.push('.')
                        }
                        ui.label(running_text);
                    }
                });

                self.update_output(ui);
            });
        });
    }

    fn setup(&mut self, ctx: &CtxRef, _: &mut epi::Frame<'_>, _: Option<&dyn epi::Storage>) {
        let mut base_style = (*ctx.style()).clone();
        base_style.spacing.text_edit_width = f32::MAX;
        base_style.spacing.item_spacing.y = 8.0;
        ctx.set_style(base_style);
    }

    fn on_exit(&mut self) {
        self.kill_child()
    }
}

impl Klask {
    fn execute(&mut self) -> Result<ChildApp, ExecuteError> {
        let args = self.state.get_cmd_args(vec![self.app.get_name().into()])?;

        // Check for validation errors
        self.app.clone().try_get_matches_from(args.iter())?;

        if self
            .env
            .as_ref()
            .and_then(|v| v.iter().find(|(key, _)| key.is_empty()))
            .is_some()
        {
            return Err("Environment variable can't be empty".into());
        }

        ChildApp::run(
            args,
            self.env.clone(),
            self.stdin.clone(),
            self.working_dir.clone(),
        )
    }

    fn update_output(&mut self, ui: &mut Ui) {
        match &mut self.output {
            Some(Ok(c)) => ui.ansi_label(c.read()),
            Some(Err(err)) => {
                ui.colored_label(Color32::RED, err.to_string());
            }
            _ => {}
        }
    }

    fn kill_child(&mut self) {
        if let Some(Ok(child)) = &mut self.output {
            child.kill();
        }
    }

    fn is_child_running(&self) -> bool {
        match &self.output {
            Some(Ok(c)) => c.is_running(),
            _ => false,
        }
    }

    fn update_env(&mut self, ui: &mut Ui) {
        let env = self.env.as_mut().unwrap();
        let mut remove_index = None;

        if !env.is_empty() {
            Grid::new(Tab::Env)
                .striped(true)
                // We can't just divide by 2, without taking spacing into account
                // Instead we just set num_columns, and the second column will fill
                .min_col_width(ui.available_width() / 3.0)
                .num_columns(2)
                .show(ui, |ui| {
                    for (index, (key, value)) in env.iter_mut().enumerate() {
                        let left = ui.horizontal(|ui| {
                            let clicked = ui.small_button("-").clicked();

                            let previous = key.is_empty().then(|| klask_ui::set_error_style(ui));
                            ui.text_edit_singleline(key);
                            if let Some(previous) = previous {
                                ui.set_style(previous);
                            }

                            clicked
                        });

                        if left.inner {
                            remove_index = Some(index);
                        }

                        ui.horizontal(|ui| {
                            ui.label("=");
                            ui.text_edit_singleline(value);
                        });

                        ui.end_row();
                    }
                });
        }

        if let Some(remove_index) = remove_index {
            env.remove(remove_index);
        }

        if ui.button("New").clicked() {
            env.push(Default::default());
        }

        ui.separator();
    }

    fn update_stdin(&mut self, ui: &mut Ui) {
        let stdin = self.stdin.as_mut().unwrap();
        ui.columns(2, |ui| {
            if ui[0]
                .selectable_label(matches!(stdin, StdinType::Text(_)), "Text")
                .clicked()
                && matches!(stdin, StdinType::File(_))
            {
                *stdin = StdinType::Text(String::new());
            }
            if ui[1]
                .selectable_label(matches!(stdin, StdinType::File(_)), "File")
                .clicked()
                && matches!(stdin, StdinType::Text(_))
            {
                *stdin = StdinType::File(String::new());
            }
        });

        match stdin {
            StdinType::File(path) => {
                ui.horizontal(|ui| {
                    if ui.button("Select file...").clicked() {
                        if let Some(file) = FileDialog::new().show_open_single_file().ok().flatten()
                        {
                            *path = file.to_string_lossy().into_owned();
                        }
                    }
                    ui.text_edit_singleline(path);
                });
            }
            StdinType::Text(text) => {
                ui.text_edit_multiline(text);
            }
        };
    }
}
