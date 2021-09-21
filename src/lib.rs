#![feature(command_access)] // only for debugging
pub mod example_opts;

use clap::{App, ArgMatches, FromArgMatches, IntoApp};
use eframe::{
    egui::{self, Button, TextEdit, Ui},
    epi,
};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread::{self},
};

pub struct Klask {
    name: String,
    child: Option<(Child, Receiver<String>)>,
    output: String,
    state: AppState,
}

impl Klask {
    pub fn run_derived<C, F>(f: F)
    where
        C: IntoApp + FromArgMatches,
        F: FnOnce(C),
    {
        Self::run_app(C::into_app(), |m| f(C::from_arg_matches(m).unwrap()));
    }

    pub fn run_app(app: App, f: impl FnOnce(&ArgMatches)) {
        match App::new("Outer GUI")
            .subcommand(app.clone())
            .try_get_matches()
            .expect("Arguments should've been verified by the GUI app")
            .subcommand_matches(app.get_name())
        {
            Some(m) => f(m),
            None => {
                let klask = Self {
                    name: app.get_name().to_string(),
                    child: None,
                    output: String::new(),
                    state: AppState::new(&app),
                };
                let native_options = eframe::NativeOptions::default();
                eframe::run_native(Box::new(klask), native_options);
            }
        }
    }
}

impl epi::App for Klask {
    fn name(&self) -> &str {
        &self.name
    }

    fn update(&mut self, ctx: &eframe::egui::CtxRef, _frame: &mut epi::Frame<'_>) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::auto_sized().show(ui, |ui| {
                self.state.update(ui);

                ui.horizontal(|ui| {
                    if ui
                        .add(Button::new("Run!").enabled(self.child.is_none()))
                        .clicked()
                    {
                        let mut cmd = Command::new(std::env::current_exe().unwrap());
                        cmd.stdout(Stdio::piped()).arg(&self.name);
                        match self.state.set_args(cmd) {
                            Ok(mut cmd) => {
                                // let args: Vec<_> = cmd.get_args().collect();
                                // println!("{:?}", args);
                                self.output = String::new();

                                let mut child = cmd.spawn().unwrap();
                                let mut reader = BufReader::new(child.stdout.take().unwrap());

                                let (tx, rx) = mpsc::channel();
                                thread::spawn(move || loop {
                                    let mut output = String::new();
                                    if let Ok(0) = reader.read_line(&mut output) {
                                        break;
                                    } else {
                                        tx.send(output).unwrap();
                                    }
                                });

                                self.child = Some((child, rx));
                            }
                            Err(()) => {
                                self.output = String::from("Incorrect");
                            }
                        }
                    }

                    if let Some((child, _)) = &mut self.child {
                        if ui.button("Kill").clicked() {
                            let _ = child.kill();
                            self.child = None;
                        }
                    }
                });

                if let Some((_, receiver)) = &mut self.child {
                    for line in receiver.try_iter() {
                        self.output.push_str(&line);
                    }
                }

                ui.label(&self.output);
            });
        });
    }

    fn on_exit(&mut self) {
        if let Some((child, _)) = &mut self.child {
            let _ = child.kill();
        }
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
    required: bool,
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
                let desc = if let Some(about) = a.get_long_about() {
                    Some(about.to_string())
                } else if let Some(about) = a.get_about() {
                    Some(about.to_string())
                } else {
                    None
                };

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

                ArgState {
                    name: a.get_name().to_string(),
                    call_name: a
                        .get_long()
                        .map(|s| format!("--{}", s))
                        .or(a.get_short().map(|c| format!("-{}", c))),
                    desc,
                    required: a.is_set(clap::ArgSettings::Required),
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

    fn set_args(&self, mut cmd: Command) -> Result<Command, ()> {
        for ArgState {
            call_name,
            kind,
            required,
            ..
        } in &self.args
        {
            match kind {
                ArgKind::String { value, default } => {
                    if let Some(call_name) = call_name.as_ref() {
                        cmd.arg(call_name);
                    }

                    match (&value[..], default, required) {
                        ("", None, true) => return Err(()),
                        ("", None, false) => {}
                        ("", Some(default), _) => {
                            cmd.arg(default);
                        }
                        (value, _, _) => {
                            cmd.arg(value);
                        }
                    };
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
            self.subcommands[current].set_args(cmd)
        } else if self.subcommands.len() > 0 {
            Err(())
        } else {
            Ok(cmd)
        }
    }
}
