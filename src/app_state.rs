use crate::arg_state::ArgState;
use clap::App;
use eframe::egui::Ui;
use std::{collections::HashMap, process::Command};

pub struct AppState {
    about: Option<String>,
    args: Vec<ArgState>,
    subcommands: HashMap<String, AppState>,
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
            current: None,
        }
    }

    pub fn update(&mut self, ui: &mut Ui) {
        if let Some(ref about) = self.about {
            ui.label(about);
        }

        for arg in &mut self.args {
            arg.update(ui);
        }

        ui.horizontal(|ui| {
            for name in self.subcommands.keys() {
                ui.selectable_value(&mut self.current, Some(name.clone()), name);
            }
        });

        if let Some(current) = &self.current {
            self.subcommands.get_mut(current).unwrap().update(ui);
        }
    }

    pub fn cmd_args(&self, mut cmd: Command) -> Result<Command, ()> {
        for arg in &self.args {
            cmd = arg.cmd_args(cmd)?;
        }

        if let Some(current) = &self.current {
            cmd.arg(current);
            self.subcommands[current].cmd_args(cmd)
        } else if !self.subcommands.is_empty() {
            Err(())
        } else {
            Ok(cmd)
        }
    }
}
