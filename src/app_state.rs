use crate::{arg_state::ArgState, settings::Localization};
use clap::Command;
use eframe::egui::{widgets::Widget, Grid, Response, Ui};
use inflector::Inflector;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AppState<'s> {
    id: Uuid,
    about: Option<String>,
    args: Vec<ArgState<'s>>,
    subcommands: BTreeMap<String, AppState<'s>>,
    current: Option<String>,
}

impl<'s> AppState<'s> {
    pub fn new(app: &Command, localization: &'s Localization) -> Self {
        let args = app
            .get_arguments()
            .filter(|a| a.get_id() != "help" && a.get_id() != "version")
            .map(|a| ArgState::new(a, localization))
            .collect();

        let subcommands = app
            .get_subcommands()
            .map(|app| (app.get_name().to_string(), AppState::new(app, localization)))
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

    pub fn update_validation_error(&mut self, name: &str, message: &str) {
        for arg in &mut self.args {
            arg.update_validation_error(name, message);
        }

        if let Some(current) = &self.current {
            self.subcommands
                .get_mut(current)
                .unwrap()
                .update_validation_error(name, message);
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

impl Widget for &mut AppState<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
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
                            ui.add(arg);
                            ui.end_row();
                        }
                    });
            }

            ui.separator();

            if !self.subcommands.is_empty() {
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
            }

            if let Some(current) = &self.current {
                ui.add(self.subcommands.get_mut(current).unwrap());
            }
        })
        .response
    }
}

#[cfg(test)]
mod tests;
