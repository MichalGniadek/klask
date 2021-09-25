use crate::{arg_state::ArgState, ValidationErrorInfo};
use clap::App;
use eframe::egui::{Grid, Ui};
use inflector::Inflector;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AppState {
    id: Uuid,
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
            id: Uuid::new_v4(),
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

        // Even empty grid adds an empty line
        if !self.args.is_empty() {
            Grid::new(self.id)
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    for arg in &mut self.args {
                        arg.update(ui, validation_error);
                        ui.end_row();
                    }
                });
        }

        ui.separator();

        // It probably should be changed to wrapping when there are more than a few
        ui.columns(self.subcommands.len(), |ui| {
            for (i, name) in self.subcommands.keys().enumerate() {
                ui[i].selectable_value(
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

    pub fn get_cmd_args(&self, mut args: Vec<String>) -> Result<Vec<String>, String> {
        for arg in &self.args {
            args = arg.get_cmd_args(args)?;
        }

        if let Some(current) = &self.current {
            args.push(current.clone());
            self.subcommands[current].get_cmd_args(args)
        } else {
            Ok(args)
        }
    }
}
