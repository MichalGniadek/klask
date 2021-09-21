#![feature(command_access)]
pub mod example_opts;

use clap::App;
use eframe::{
    egui::{self, TextEdit, Ui},
    epi,
};
use std::{
    collections::HashMap,
    process::{Command, Stdio},
};

pub struct Klask {
    name: String,
    output: Option<String>,
    state: AppState,
}

impl Klask {
    pub fn new(app: App) -> Self {
        Self {
            name: app.get_name().to_string(),
            output: None,
            state: AppState::new(&app),
        }
    }
}

impl epi::App for Klask {
    fn name(&self) -> &str {
        &self.name
    }

    fn update(&mut self, ctx: &eframe::egui::CtxRef, _frame: &mut epi::Frame<'_>) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.state.update(ui);

            if ui.button("Run!").clicked() {
                let mut cmd = Command::new("./target/debug/klask");
                cmd.stdout(Stdio::piped());
                self.state.set_args(&mut cmd);
                // let args: Vec<_> = cmd.get_args().collect();
                // println!("{:?}", args);
                let out = cmd.spawn().unwrap().wait_with_output().unwrap();
                self.output = Some(std::str::from_utf8(&out.stdout).unwrap().to_string());
            }

            if let Some(output) = &self.output {
                ui.label(output);
            }
        });
    }
}

struct AppState {
    about: Option<String>,
    args: Vec<ArgState>,
    subcommands: HashMap<String, AppState>,
    current: Option<String>,
}

pub struct ArgState {
    name: String,
    call_name: Option<String>,
    desc: Option<String>,
    _required: bool,
    kind: ArgKind,
}

pub enum ArgKind {
    String {
        value: String,
        default: Option<String>,
    },
    Occurences(i32),
    Bool(bool),
}

impl AppState {
    pub fn new(app: &App) -> Self {
        let args = app
            .get_arguments()
            .filter(|a| a.get_name() != "help" && a.get_name() != "version")
            .map(|a| {
                let kind = if a.is_set(clap::ArgSettings::MultipleOccurrences) {
                    ArgKind::Occurences(0)
                } else if !a.is_set(clap::ArgSettings::TakesValue) {
                    ArgKind::Bool(false)
                } else {
                    ArgKind::String {
                        value: "".into(),
                        default: a
                            .get_default_values()
                            .first()
                            .map(|s| s.to_string_lossy().into_owned()),
                    }
                };

                let desc = if let Some(about) = a.get_long_about() {
                    Some(about.to_string())
                } else if let Some(about) = a.get_about() {
                    Some(about.to_string())
                } else {
                    None
                };

                ArgState {
                    name: a.get_name().to_string(),
                    call_name: a
                        .get_long()
                        .map(|s| format!("--{}", s))
                        .or(a.get_short().map(|c| format!("-{}", c))),
                    desc,
                    _required: a.is_set(clap::ArgSettings::Required),
                    kind,
                }
            })
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

        for ArgState {
            ref name,
            desc,
            kind,
            ..
        } in &mut self.args
        {
            ui.horizontal(|ui| {
                let label = ui.label(name);

                if let Some(desc) = desc {
                    label.on_hover_text(desc);
                }

                match kind {
                    ArgKind::String { value, default } => {
                        ui.add(
                            TextEdit::singleline(value)
                                .hint_text(default.as_ref().unwrap_or(&String::new()))
                                .desired_width(f32::MAX),
                        );
                    }
                    ArgKind::Occurences(i) => {
                        ui.horizontal(|ui| {
                            if ui.small_button("-").clicked() {
                                *i = (*i - 1).max(0);
                            }
                            ui.label(i.to_string());
                            if ui.small_button("+").clicked() {
                                *i += 1;
                            }
                        });
                    }
                    ArgKind::Bool(bool) => {
                        ui.checkbox(bool, "");
                    }
                };
            });
        }

        ui.horizontal(|ui| {
            for (name, _) in &self.subcommands {
                ui.selectable_value(&mut self.current, Some(name.clone()), name);
            }
        });

        if let Some(current) = &self.current {
            self.subcommands.get_mut(current).unwrap().update(ui);
        }
    }

    fn set_args(&self, cmd: &mut Command) {
        for ArgState {
            call_name, kind, ..
        } in &self.args
        {
            match kind {
                ArgKind::String { value, default } => {
                    if let Some(call_name) = call_name.as_ref() {
                        cmd.arg(call_name);
                    }

                    if let Some(default) = default {
                        if value == "" {
                            cmd.arg(default);
                        } else {
                            cmd.arg(value);
                        }
                    } else {
                        cmd.arg(value);
                    }
                }
                &ArgKind::Occurences(i) => {
                    for _ in 0..i {
                        cmd.arg(call_name.as_ref().unwrap());
                    }
                }
                &ArgKind::Bool(bool) => {
                    if bool {
                        cmd.arg(call_name.as_ref().unwrap());
                    }
                }
            }
        }

        if let Some(current) = &self.current {
            cmd.arg(current);
            self.subcommands[current].set_args(cmd);
        }
    }
}
