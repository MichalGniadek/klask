#![warn(missing_docs)]
//! You can use [`run_app`] for [`App`]s created manually or generated from yaml and
//! [`run_derived`] for [`App`]s derived from a struct. Both of these functions take
//! a closure that contains the code that would normally be in `main`. They should be
//! the last thing you call in `main`.
//!
//! For example
//! ```no_run
//! # use clap::{App, Arg};
//! # use klask::Settings;
//! fn main() {
//!     let app = App::new("Example").arg(Arg::new("debug").short('d'));
//!     klask::run_app(app, Settings::default(), |matches| {
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
/// Additional options for output like progress bars.
pub mod output;
mod settings;

use app_state::AppState;
use child_app::{ChildApp, StdinType};
use clap::{App, ArgMatches, FromArgMatches, IntoApp};
use eframe::{
    egui::{self, style::Spacing, Button, Color32, CtxRef, Grid, Style, TextEdit, Ui},
    epi,
};
use error::ExecuteError;
use native_dialog::FileDialog;

pub use settings::Settings;
use std::hash::Hash;

/// Call with an [`App`] and a closure that contains the code that would normally be in `main`.
/// ```no_run
/// # use clap::{App, Arg};
/// # use klask::Settings;
/// let app = App::new("Example").arg(Arg::new("debug").short('d'));

/// klask::run_app(app, Settings::default(), |matches| {
///    println!("{}", matches.is_present("debug"))
/// });
/// ```
pub fn run_app(app: App<'static>, settings: Settings, f: impl FnOnce(&ArgMatches)) {
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
                    env: settings.enable_env.map(|desc| (desc, vec![])),
                    stdin: settings
                        .enable_stdin
                        .map(|desc| (desc, StdinType::Text(String::new()))),
                    working_dir: settings
                        .enable_working_dir
                        .map(|desc| (desc, String::new())),
                    output: None,
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
/// # use klask::Settings;
/// #[derive(Clap)]
/// struct Example {
///     #[clap(short)]
///     debug: bool,
/// }
///
/// klask::run_derived::<Example, _>(Settings::default(), |example|{
///     println!("{}", example.debug);
/// });
/// ```
pub fn run_derived<C, F>(settings: Settings, f: F)
where
    C: IntoApp + FromArgMatches,
    F: FnOnce(C),
{
    run_app(C::into_app(), settings, |m| {
        let matches = C::from_arg_matches(m)
            .expect("Internal error, C::from_arg_matches should always succeed");
        f(matches);
    });
}

#[derive(Debug)]
struct Klask {
    state: AppState,
    tab: Tab,
    /// First string is a description
    env: Option<(String, Vec<(String, String)>)>,
    /// First string is a description
    stdin: Option<(String, StdinType)>,
    /// First string is a description
    working_dir: Option<(String, String)>,
    output: Option<Result<ChildApp, ExecuteError>>,
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
                // Tab selection
                let tab_count = 1
                    + if self.env.is_some() { 1 } else { 0 }
                    + if self.stdin.is_some() { 1 } else { 0 };

                if tab_count > 1 {
                    ui.columns(tab_count, |ui| {
                        let mut index = 0;

                        ui[index].selectable_value(&mut self.tab, Tab::Arguments, "Arguments");
                        index += 1;

                        if self.env.is_some() {
                            ui[index].selectable_value(
                                &mut self.tab,
                                Tab::Env,
                                "Environment variables",
                            );
                            index += 1;
                        }
                        if self.stdin.is_some() {
                            ui[index].selectable_value(&mut self.tab, Tab::Stdin, "Input");
                        }
                    });

                    ui.separator();
                }

                // Display selected tab
                match self.tab {
                    Tab::Arguments => {
                        ui.add(&mut self.state);

                        // Working dir
                        if let Some((ref desc, path)) = &mut self.working_dir {
                            if !desc.is_empty() {
                                ui.label(desc);
                            }

                            ui.horizontal(|ui| {
                                if ui.button("Select directory...").clicked() {
                                    if let Some(file) =
                                        FileDialog::new().show_open_single_dir().ok().flatten()
                                    {
                                        *path = file.to_string_lossy().into_owned();
                                    }
                                }
                                ui.add(TextEdit::singleline(path).hint_text("Working directory"))
                            });
                            ui.add_space(10.0);
                        }
                    }
                    Tab::Env => self.update_env(ui),
                    Tab::Stdin => self.update_stdin(ui),
                }

                // Run button row
                ui.horizontal(|ui| {
                    if ui
                        .add(Button::new("Run!").enabled(!self.is_child_running()))
                        .clicked()
                    {
                        let output = self.execute();

                        if let Err(ExecuteError::ValidationError { name, message }) = &output {
                            self.state.update_validation_error(name, message);
                        } else {
                            // Reset
                            self.state.update_validation_error("", "");
                        }

                        self.output = Some(output);
                    }

                    if self.is_child_running() && ui.button("Kill").clicked() {
                        self.kill_child();
                    }

                    if let Some(Ok(child)) = &self.output {
                        if ui.button("Copy output").clicked() {
                            ctx.output().copied_text = child.output.get_output_string();
                        }
                    }

                    if self.is_child_running() {
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
        ctx.set_style(Klask::klask_style());
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
            .and_then(|(_, v)| v.iter().find(|(key, _)| key.is_empty()))
            .is_some()
        {
            return Err("Environment variable can't be empty".into());
        }

        ChildApp::run(
            args,
            self.env.clone().map(|(_, env)| env),
            self.stdin.clone().map(|(_, stdin)| stdin),
            self.working_dir.clone().map(|(_, dir)| dir),
        )
    }

    fn update_output(&mut self, ui: &mut Ui) {
        match &mut self.output {
            Some(Ok(c)) => c.read().update(ui),
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
        let (ref desc, env) = self.env.as_mut().unwrap();

        if !desc.is_empty() {
            ui.label(desc);
        }

        if !env.is_empty() {
            let mut remove_index = None;

            Grid::new(Tab::Env)
                .striped(true)
                // We can't just divide by 2, without taking spacing into account
                // Instead we just set num_columns, and the second column will fill
                .min_col_width(ui.available_width() / 3.0)
                .num_columns(2)
                .show(ui, |ui| {
                    for (index, (key, value)) in env.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            if ui.small_button("-").clicked() {
                                remove_index = Some(index);
                            }

                            if key.is_empty() {
                                ui.set_style(Klask::error_style());
                            }

                            ui.text_edit_singleline(key);

                            if key.is_empty() {
                                ui.set_style(Klask::klask_style());
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("=");
                            ui.text_edit_singleline(value);
                        });

                        ui.end_row();
                    }
                });

            if let Some(remove_index) = remove_index {
                env.remove(remove_index);
            }
        }

        if ui.button("New").clicked() {
            env.push(Default::default());
        }

        ui.separator();
    }

    fn update_stdin(&mut self, ui: &mut Ui) {
        let (ref desc, stdin) = self.stdin.as_mut().unwrap();

        if !desc.is_empty() {
            ui.label(desc);
        }

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

    fn klask_style() -> Style {
        Style {
            spacing: Spacing {
                text_edit_width: f32::MAX,
                item_spacing: egui::vec2(8.0, 8.0),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn error_style() -> Style {
        let mut style = Self::klask_style();
        style.visuals.widgets.inactive.bg_stroke.color = Color32::RED;
        style.visuals.widgets.inactive.bg_stroke.width = 1.0;
        style.visuals.widgets.hovered.bg_stroke.color = Color32::RED;
        style.visuals.widgets.active.bg_stroke.color = Color32::RED;
        style.visuals.widgets.open.bg_stroke.color = Color32::RED;
        style.visuals.widgets.noninteractive.bg_stroke.color = Color32::RED;
        style.visuals.selection.stroke.color = Color32::RED;
        style
    }
}
