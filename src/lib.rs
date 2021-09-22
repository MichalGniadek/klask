#![feature(command_access)]
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

pub struct Klask {
    output: Option<Result<ChildApp, ExecuteError>>,
    state: AppState,
    validation_error: Option<ValidationErrorInfo>,
    // This isn't a generic lifetime because eframe::run_native() requires
    // a 'static lifetime because boxed trait objects default to 'static
    app: App<'static>,
}

// Public interface
impl Klask {
    pub fn run_app(app: App<'static>, f: impl FnOnce(&ArgMatches)) {
        // Wrap app in another in case no arguments is a valid configuration
        match App::new("outer").subcommand(app.clone()).try_get_matches() {
            Ok(matches) => match matches.subcommand_matches(app.get_name()) {
                // Called with arguments -> start user program
                Some(m) => f(m),
                // Called with no arguments -> start gui
                None => {
                    let klask = Self {
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

    pub fn run_derived<C, F>(f: F)
    where
        C: IntoApp + FromArgMatches,
        F: FnOnce(C),
    {
        Self::run_app(C::into_app(), |m| {
            let matches = C::from_arg_matches(m)
                .expect("Internal error, C::from_arg_matches should always succeed");

            f(matches);
        });
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
