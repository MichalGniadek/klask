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
use clap::{ArgMatches, Command, FromArgMatches, IntoApp};
use eframe::{
    egui::{self, Button, Color32, Context, FontData, FontDefinitions, Grid, Style, TextEdit, Ui},
    CreationContext, Frame,
};
use error::ExecutionError;
use rfd::FileDialog;

use output::Output;
pub use settings::{Localization, Settings};
use std::{borrow::Cow, hash::Hash};

const CHILD_APP_ENV_VAR: &str = "KLASK_CHILD_APP";

/// Call with an [`App`] and a closure that contains the code that would normally be in `main`.
/// ```no_run
/// # use clap::{App, Arg};
/// # use klask::Settings;
/// let app = App::new("Example").arg(Arg::new("debug").short('d'));

/// klask::run_app(app, Settings::default(), |matches| {
///    println!("{}", matches.is_present("debug"))
/// });
/// ```
pub fn run_app(app: Command<'static>, settings: Settings, f: impl FnOnce(&ArgMatches)) {
    if std::env::var(CHILD_APP_ENV_VAR).is_ok() {
        std::env::remove_var(CHILD_APP_ENV_VAR);

        let matches = app
            .try_get_matches()
            .expect("Internal error, arguments should've been verified by the GUI app");

        f(&matches);
    } else {
        // During validation we don't pass in a binary name
        let app = app.setting(clap::AppSettings::NoBinaryName);
        let app_name = app.get_name().to_string();

        // eframe::run_native requires that Box::new(klask) has 'static
        // lifetime, so we must leak here. But it never returns (return value !)
        // so it should be ok.
        let localization = Box::leak(Box::new(settings.localization));

        let mut klask = Klask {
            state: AppState::new(&app, localization),
            tab: Tab::Arguments,
            env: settings.enable_env.map(|desc| (desc, vec![])),
            stdin: settings
                .enable_stdin
                .map(|desc| (desc, StdinType::Text(String::new()))),
            working_dir: settings
                .enable_working_dir
                .map(|desc| (desc, String::new())),
            output: Output::None,
            app,
            custom_font: settings.custom_font,
            localization,
            style: settings.style,
        };
        let native_options = eframe::NativeOptions::default();
        eframe::run_native(
            app_name.as_str(),
            native_options,
            Box::new(|cc| {
                klask.setup(cc);
                Box::new(klask)
            }),
        );
    }
}

/// Can be used with a struct deriving [`clap::Clap`]. Call with a closure that contains the code that would normally be in `main`.
/// It's just a wrapper over [`run_app`].
/// ```no_run
/// # use clap::{App, Arg, Parser};
/// # use klask::Settings;
/// #[derive(Parser)]
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
    run_app(C::command(), settings, |m| {
        let matches = C::from_arg_matches(m)
            .expect("Internal error, C::from_arg_matches should always succeed");
        f(matches);
    });
}

#[derive(Debug)]
struct Klask<'s> {
    state: AppState<'s>,
    tab: Tab,
    /// First string is a description
    env: Option<(String, Vec<(String, String)>)>,
    /// First string is a description
    stdin: Option<(String, StdinType)>,
    /// First string is a description
    working_dir: Option<(String, String)>,
    output: Output,
    // This isn't a generic lifetime because eframe::run_native() requires
    // a 'static lifetime because boxed trait objects default to 'static
    app: Command<'static>,

    custom_font: Option<Cow<'static, [u8]>>,
    localization: &'s Localization,
    style: Style,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum Tab {
    Arguments,
    Env,
    Stdin,
}

impl eframe::App for Klask<'_> {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Tab selection
                let tab_count =
                    1 + usize::from(self.env.is_some()) + usize::from(self.stdin.is_some());

                if tab_count > 1 {
                    ui.columns(tab_count, |ui| {
                        let mut index = 0;

                        ui[index].selectable_value(
                            &mut self.tab,
                            Tab::Arguments,
                            &self.localization.arguments,
                        );
                        index += 1;

                        if self.env.is_some() {
                            ui[index].selectable_value(
                                &mut self.tab,
                                Tab::Env,
                                &self.localization.env_variables,
                            );
                            index += 1;
                        }
                        if self.stdin.is_some() {
                            ui[index].selectable_value(
                                &mut self.tab,
                                Tab::Stdin,
                                &self.localization.input,
                            );
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

                            let localization = self.localization;
                            ui.horizontal(|ui| {
                                if ui.button(&localization.select_directory).clicked() {
                                    if let Some(file) = FileDialog::new().pick_folder() {
                                        *path = file.to_string_lossy().into_owned();
                                    }
                                }
                                ui.add(
                                    TextEdit::singleline(path)
                                        .hint_text(&localization.working_directory),
                                )
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
                        .add_enabled(
                            !self.is_child_running(),
                            Button::new(&self.localization.run),
                        )
                        .clicked()
                    {
                        match self.try_start_execution(ctx.clone()) {
                            Ok(child) => {
                                // Reset
                                self.state.update_validation_error("", "");
                                self.output = Output::new_with_child(child);
                            }
                            Err(err) => {
                                if let ExecutionError::ValidationError { name, message } = &err {
                                    self.state.update_validation_error(name, message);
                                }
                                self.output = Output::Err(err);
                            }
                        }
                    }

                    if self.is_child_running() && ui.button(&self.localization.kill).clicked() {
                        self.kill_child();
                    }

                    if self.is_child_running() {
                        let mut running_text = String::from(&self.localization.running);
                        for _ in 0..((2.0 * ui.input().time) as i32 % 4) {
                            running_text.push('.');
                        }
                        ui.label(running_text);
                    }
                });

                ui.add(&mut self.output);
            });
        });
    }
}

impl Klask<'_> {
    fn setup(&mut self, cc: &CreationContext) {
        cc.egui_ctx.set_style(self.style.clone());

        if let Some(custom_font) = self.custom_font.take() {
            let font_name = String::from("custom_font");
            let mut fonts = FontDefinitions::default();

            fonts.font_data.insert(
                font_name.clone(),
                FontData {
                    font: custom_font,
                    index: 0,
                    tweak: Default::default(),
                },
            );

            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, font_name.clone());

            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push(font_name);

            cc.egui_ctx.set_fonts(fonts);
        }
    }

    fn try_start_execution(&mut self, ctx: egui::Context) -> Result<ChildApp, ExecutionError> {
        let args = self.state.get_cmd_args(vec![])?;

        // Check for validation errors
        self.app.try_get_matches_from_mut(args.iter())?;

        if self
            .env
            .as_ref()
            .and_then(|(_, v)| v.iter().find(|(key, _)| key.is_empty()))
            .is_some()
        {
            return Err(self
                .localization
                .error_env_var_cant_be_empty
                .as_str()
                .into());
        }

        ChildApp::run(
            args,
            self.env.clone().map(|(_, env)| env),
            self.stdin.clone().map(|(_, stdin)| stdin),
            self.working_dir.clone().map(|(_, dir)| dir),
            ctx,
        )
    }

    fn kill_child(&mut self) {
        if let Output::Child(child, _) = &mut self.output {
            child.kill();
        }
    }

    fn is_child_running(&self) -> bool {
        match &self.output {
            Output::Child(child, _) => child.is_running(),
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
                                Klask::set_error_style(ui);
                            }

                            ui.text_edit_singleline(key);

                            if key.is_empty() {
                                ui.reset_style();
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

        if ui.button(&self.localization.new_value).clicked() {
            env.push(Default::default());
        }

        ui.separator();
    }

    fn update_stdin(&mut self, ui: &mut Ui) {
        let (ref desc, stdin) = self.stdin.as_mut().unwrap();

        if !desc.is_empty() {
            ui.label(desc);
        }

        let localization = &self.localization;

        ui.columns(2, |ui| {
            if ui[0]
                .selectable_label(matches!(stdin, StdinType::Text(_)), &localization.text)
                .clicked()
                && matches!(stdin, StdinType::File(_))
            {
                *stdin = StdinType::Text(String::new());
            }
            if ui[1]
                .selectable_label(matches!(stdin, StdinType::File(_)), &localization.file)
                .clicked()
                && matches!(stdin, StdinType::Text(_))
            {
                *stdin = StdinType::File(String::new());
            }
        });

        match stdin {
            StdinType::File(path) => {
                ui.horizontal(|ui| {
                    if ui.button(&localization.select_file).clicked() {
                        if let Some(file) = FileDialog::new().pick_file() {
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

    fn set_error_style(ui: &mut Ui) {
        let mut style = ui.style_mut();
        style.visuals.widgets.inactive.bg_stroke.color = Color32::RED;
        style.visuals.widgets.inactive.bg_stroke.width = 1.0;
        style.visuals.widgets.hovered.bg_stroke.color = Color32::RED;
        style.visuals.widgets.active.bg_stroke.color = Color32::RED;
        style.visuals.widgets.open.bg_stroke.color = Color32::RED;
        style.visuals.widgets.noninteractive.bg_stroke.color = Color32::RED;
        style.visuals.selection.stroke.color = Color32::RED;
    }
}
