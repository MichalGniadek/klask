#![feature(command_access)] // only for debugging
pub mod example_opts;

use cansi::{CategorisedSlice, Color};
use clap::{App, ArgMatches, ArgSettings, FromArgMatches, IntoApp, ValueHint};
use eframe::{
    egui::{self, Button, Color32, Label, TextEdit, Ui},
    epi,
};
use linkify::{LinkFinder, LinkKind};
use native_dialog::FileDialog;
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

                        ui.label("Running...");
                    }
                });

                if let Some((_, receiver)) = &mut self.child {
                    for line in receiver.try_iter() {
                        self.output.push_str(&line);
                    }
                }

                let output = cansi::categorise_text(&self.output);
                for CategorisedSlice {
                    text,
                    fg_colour,
                    bg_colour,
                    intensity,
                    italic,
                    underline,
                    strikethrough,
                    ..
                } in output
                {
                    for span in LinkFinder::new().spans(text) {
                        match span.kind() {
                            Some(LinkKind::Url) => ui.hyperlink(span.as_str()),
                            Some(LinkKind::Email) => {
                                ui.hyperlink(format!("mailto:{}", span.as_str()))
                            }
                            Some(_) | None => {
                                fn convert(color: Color) -> Color32 {
                                    match color {
                                        Color::Black => Color32::from_rgb(0, 0, 0),
                                        Color::Red => Color32::from_rgb(205, 49, 49),
                                        Color::Green => Color32::from_rgb(13, 188, 121),
                                        Color::Yellow => Color32::from_rgb(229, 229, 16),
                                        Color::Blue => Color32::from_rgb(36, 114, 200),
                                        Color::Magenta => Color32::from_rgb(188, 63, 188),
                                        Color::Cyan => Color32::from_rgb(17, 168, 205),
                                        Color::White => Color32::from_rgb(229, 229, 229),
                                        Color::BrightBlack => Color32::from_rgb(102, 102, 102),
                                        Color::BrightRed => Color32::from_rgb(241, 76, 76),
                                        Color::BrightGreen => Color32::from_rgb(35, 209, 139),
                                        Color::BrightYellow => Color32::from_rgb(245, 245, 67),
                                        Color::BrightBlue => Color32::from_rgb(59, 142, 234),
                                        Color::BrightMagenta => Color32::from_rgb(214, 112, 214),
                                        Color::BrightCyan => Color32::from_rgb(41, 184, 219),
                                        Color::BrightWhite => Color32::from_rgb(229, 229, 229),
                                    }
                                }

                                let mut label =
                                    Label::new(span.as_str()).text_color(convert(fg_colour));

                                if bg_colour != Color::Black {
                                    label = label.background_color(convert(bg_colour));
                                }

                                if italic {
                                    label = label.italics();
                                }

                                if underline {
                                    label = label.underline();
                                }

                                if strikethrough {
                                    label = label.strikethrough();
                                }

                                label = match intensity {
                                    cansi::Intensity::Normal => label,
                                    cansi::Intensity::Bold => label.strong(),
                                    cansi::Intensity::Faint => label.weak(),
                                };

                                ui.add(label)
                            }
                        };
                    }
                }
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
    MultipleStrings {
        values: Vec<String>,
        default: Vec<String>,
    },
    Occurences(i32),
    Bool(bool),
    Path {
        value: String,
        default: Option<String>,
        allow_dir: bool,
        allow_file: bool,
    },
    MultiplePaths {
        values: Vec<String>,
        default: Vec<String>,
        allow_dir: bool,
        allow_file: bool,
    },
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

                let kind = match (
                    a.is_set(ArgSettings::MultipleOccurrences),
                    a.is_set(ArgSettings::TakesValue),
                    a.get_value_hint(),
                ) {
                    (true, true, ValueHint::AnyPath | ValueHint::DirPath | ValueHint::FilePath) => {
                        let default: Vec<_> = a
                            .get_default_values()
                            .iter()
                            .map(|s| s.to_string_lossy().into_owned())
                            .collect();
                        ArgKind::MultiplePaths {
                            values: default.clone(),
                            default,
                            allow_dir: matches!(
                                a.get_value_hint(),
                                ValueHint::AnyPath | ValueHint::DirPath
                            ),
                            allow_file: matches!(
                                a.get_value_hint(),
                                ValueHint::AnyPath | ValueHint::FilePath
                            ),
                        }
                    }
                    (true, true, _) => {
                        let default: Vec<_> = a
                            .get_default_values()
                            .iter()
                            .map(|s| s.to_string_lossy().into_owned())
                            .collect();
                        ArgKind::MultipleStrings {
                            values: default.clone(),
                            default,
                        }
                    }
                    (true, false, _) => ArgKind::Occurences(0),
                    (
                        false,
                        true,
                        ValueHint::AnyPath | ValueHint::DirPath | ValueHint::FilePath,
                    ) => {
                        let default = a
                            .get_default_values()
                            .first()
                            .map(|s| s.to_string_lossy().into_owned());
                        ArgKind::Path {
                            value: default.clone().unwrap_or(String::new()),
                            default,
                            allow_dir: matches!(
                                a.get_value_hint(),
                                ValueHint::AnyPath | ValueHint::DirPath
                            ),
                            allow_file: matches!(
                                a.get_value_hint(),
                                ValueHint::AnyPath | ValueHint::FilePath
                            ),
                        }
                    }
                    (false, true, _) => ArgKind::String {
                        value: "".into(),
                        default: a
                            .get_default_values()
                            .first()
                            .map(|s| s.to_string_lossy().into_owned()),
                    },
                    (false, false, _) => ArgKind::Bool(false),
                };

                ArgState {
                    name: a.get_name().to_string(),
                    call_name: a
                        .get_long()
                        .map(|s| format!("--{}", s))
                        .or(a.get_short().map(|c| format!("-{}", c))),
                    desc,
                    required: a.is_set(ArgSettings::Required),
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
            required,
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
                                .hint_text(default.clone().unwrap_or_else(|| {
                                    if *required {
                                        String::new()
                                    } else {
                                        String::from("(Optional)")
                                    }
                                }))
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
                    ArgKind::MultipleStrings { values, default } => {
                        ui.vertical(|ui| {
                            let mut remove_index = None;
                            for (index, value) in values.iter_mut().enumerate() {
                                ui.horizontal(|ui| {
                                    if ui.small_button("-").clicked() {
                                        remove_index = Some(index);
                                    }
                                    ui.add(TextEdit::singleline(value).desired_width(f32::MAX));
                                });
                            }

                            if let Some(index) = remove_index {
                                values.remove(index);
                            }

                            ui.horizontal(|ui| {
                                if ui.button("New value").clicked() {
                                    values.push(String::new());
                                }
                                ui.add_space(20.0);
                                if ui.button("Reset to default").clicked() {
                                    *values = default.clone();
                                }
                            })
                        });
                    }
                    ArgKind::Path {
                        value,
                        default,
                        allow_dir,
                        allow_file,
                    } => {
                        if *allow_file && ui.button("Select file...").clicked() {
                            if let Some(file) = FileDialog::new()
                                .show_open_single_file()
                                .unwrap()
                                .map(|p| p.to_str().unwrap().to_string())
                            {
                                *value = file;
                            }
                        }

                        if *allow_dir && ui.button("Select directory...").clicked() {
                            if let Some(file) = FileDialog::new()
                                .show_open_single_dir()
                                .unwrap()
                                .map(|p| p.to_str().unwrap().to_string())
                            {
                                *value = file;
                            }
                        }

                        if let Some(default) = default {
                            ui.add_space(20.0);
                            if ui.button("Reset to default").clicked() {
                                *value = default.clone();
                            }
                        }

                        ui.add(TextEdit::singleline(value).desired_width(f32::MAX));
                    }
                    ArgKind::MultiplePaths {
                        values,
                        default,
                        allow_dir,
                        allow_file,
                    } => {
                        ui.vertical(|ui| {
                            let mut remove_index = None;
                            for (index, value) in values.iter_mut().enumerate() {
                                ui.horizontal(|ui| {
                                    if ui.small_button("-").clicked() {
                                        remove_index = Some(index);
                                    }

                                    if *allow_file && ui.button("Select file...").clicked() {
                                        if let Some(file) = FileDialog::new()
                                            .show_open_single_file()
                                            .unwrap()
                                            .map(|p| p.to_str().unwrap().to_string())
                                        {
                                            *value = file;
                                        }
                                    }

                                    if *allow_dir && ui.button("Select directory...").clicked() {
                                        if let Some(file) = FileDialog::new()
                                            .show_open_single_dir()
                                            .unwrap()
                                            .map(|p| p.to_str().unwrap().to_string())
                                        {
                                            *value = file;
                                        }
                                    }

                                    ui.add(TextEdit::singleline(value).desired_width(f32::MAX));
                                });
                            }

                            if let Some(index) = remove_index {
                                values.remove(index);
                            }

                            ui.horizontal(|ui| {
                                if ui.button("New value").clicked() {
                                    values.push(String::new());
                                }
                                ui.add_space(20.0);
                                if ui.button("Reset to default").clicked() {
                                    *values = default.clone();
                                }
                            })
                        });
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
                    match (&value[..], default, required) {
                        ("", None, true) => return Err(()),
                        ("", None, false) => {}
                        ("", Some(default), _) => {
                            if let Some(call_name) = call_name.as_ref() {
                                cmd.arg(call_name);
                            }
                            cmd.arg(default);
                        }
                        (value, _, _) => {
                            if let Some(call_name) = call_name.as_ref() {
                                cmd.arg(call_name);
                            }
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
                ArgKind::MultipleStrings { values, .. } => {
                    for value in values {
                        if let Some(call_name) = call_name.as_ref() {
                            cmd.arg(call_name);
                        }
                        cmd.arg(value);
                    }
                }
                ArgKind::Path { value, default, .. } => match (&value[..], default, required) {
                    ("", None, true) => return Err(()),
                    ("", None, false) => {}
                    ("", Some(default), _) => {
                        if let Some(call_name) = call_name.as_ref() {
                            cmd.arg(call_name);
                        }
                        cmd.arg(default);
                    }
                    (value, _, _) => {
                        if let Some(call_name) = call_name.as_ref() {
                            cmd.arg(call_name);
                        }
                        cmd.arg(value);
                    }
                },
                ArgKind::MultiplePaths { values, .. } => {
                    for value in values {
                        if let Some(call_name) = call_name.as_ref() {
                            cmd.arg(call_name);
                        }
                        cmd.arg(value);
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
