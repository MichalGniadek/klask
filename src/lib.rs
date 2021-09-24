#![feature(command_access)]
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
//! Currently requires nightly.
mod app_state;
mod arg_state;
mod child_app;
mod error;
mod klask_ui;

use app_state::AppState;
use child_app::ChildApp;
use clap::{App, ArgMatches, FromArgMatches, IntoApp};
use eframe::{
    egui::{self, Button, Color32, Ui},
    epi,
};
use error::{ExecuteError, ValidationErrorInfo};
use klask_ui::KlaskUi;
use std::process::Command;

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
                    output: None,
                    state: AppState::new(&app),
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
    output: Option<Result<ChildApp, ExecuteError>>,
    state: AppState,
    validation_error: Option<ValidationErrorInfo>,
    // This isn't a generic lifetime because eframe::run_native() requires
    // a 'static lifetime because boxed trait objects default to 'static
    app: App<'static>,
}

impl epi::App for Klask {
    fn name(&self) -> &str {
        self.app.get_name()
    }

    fn update(&mut self, ctx: &eframe::egui::CtxRef, _frame: &mut epi::Frame<'_>) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.style_mut().spacing.text_edit_width = f32::MAX;
            egui::ScrollArea::auto_sized().show(ui, |ui| {
                self.state.update(ui, &mut self.validation_error);

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

    fn on_exit(&mut self) {
        self.kill_child()
    }
}

impl Klask {
    fn execute(&mut self) -> Result<ChildApp, ExecuteError> {
        // Call the same executable, with subcommand equal to inner app's name
        let mut cmd = Command::new(std::env::current_exe()?);
        cmd.arg(self.app.get_name());
        let mut cmd = self.state.set_cmd_args(cmd)?;

        // Check for validation errors
        self.app.clone().try_get_matches_from(cmd.get_args())?;

        ChildApp::run(&mut cmd)
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
}
