use crate::{arg_state::ArgState, ValidationErrorInfo};
use clap::App;
use eframe::egui::Ui;
use inflector::Inflector;
use std::{collections::BTreeMap, process::Command};

pub struct AppState {
    about: Option<String>,
    args: Vec<ArgState>,
    subcommands: BTreeMap<String, AppState>,
    current: Option<String>,
}

impl AppState {
    pub fn new(app: &App) -> Self {
        let args = app
            .get_arguments()
            .filter(|a| a.get_name() != "help" && a.get_name() != "version")
            .map(ArgState::from)
            .collect();

        let subcommands = app
            .get_subcommands()
            .map(|app| (app.get_name().to_string(), AppState::new(app)))
            .collect();

        AppState {
            about: app.get_about().map(String::from),
            args,
            subcommands,
            current: app
                .get_subcommands()
                .map(|app| app.get_name().to_string())
                .next(),
        }
    }

    pub fn update(&mut self, ui: &mut Ui, validation_error: &mut Option<ValidationErrorInfo>) {
        if let Some(ref about) = self.about {
            ui.label(about);
        }

        for arg in &mut self.args {
            arg.update(ui, validation_error);
        }

        ui.horizontal(|ui| {
            for name in self.subcommands.keys() {
                ui.selectable_value(
                    &mut self.current,
                    Some(name.clone()),
                    name.to_sentence_case(),
                );
            }
        });

        if let Some(current) = &self.current {
            self.subcommands
                .get_mut(current)
                .unwrap()
                .update(ui, validation_error);
        }
    }

    pub fn set_cmd_args(&self, mut cmd: Command) -> Result<Command, String> {
        for arg in &self.args {
            cmd = arg.set_cmd_args(cmd)?;
        }

        if let Some(current) = &self.current {
            cmd.arg(current);
            self.subcommands[current].set_cmd_args(cmd)
        } else {
            Ok(cmd)
        }
    }
}
