use clap::{Arg, ArgSettings, ValueHint};
use eframe::egui::{TextEdit, Ui};
use native_dialog::FileDialog;
use std::process::Command;

pub struct ArgState {
    pub name: String,
    pub call_name: Option<String>,
    pub desc: Option<String>,
    pub required: bool,
    pub kind: ArgKind,
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

impl ArgState {
    pub fn update(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let label = ui.label(&self.name);

            if let Some(desc) = &self.desc {
                label.on_hover_text(desc);
            }

            match &mut self.kind {
                ArgKind::String { value, default } => {
                    ui.add(
                        TextEdit::singleline(value)
                            .hint_text(default.clone().unwrap_or(if self.required {
                                String::new()
                            } else {
                                String::from("(Optional)")
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

    pub fn cmd_args(&self, mut cmd: Command) -> Result<Command, ()> {
        match &self.kind {
            ArgKind::String { value, default } => {
                match (&value[..], default, self.required) {
                    ("", None, true) => return Err(()),
                    ("", None, false) => {}
                    ("", Some(default), _) => {
                        if let Some(call_name) = self.call_name.as_ref() {
                            cmd.arg(call_name);
                        }
                        cmd.arg(default);
                    }
                    (value, _, _) => {
                        if let Some(call_name) = self.call_name.as_ref() {
                            cmd.arg(call_name);
                        }
                        cmd.arg(value);
                    }
                };
            }
            &ArgKind::Occurences(i) => {
                for _ in 0..i {
                    cmd.arg(self.call_name.as_ref().unwrap());
                }
            }
            &ArgKind::Bool(bool) => {
                if bool {
                    cmd.arg(self.call_name.as_ref().unwrap());
                }
            }
            ArgKind::MultipleStrings { values, .. } => {
                for value in values {
                    if let Some(call_name) = self.call_name.as_ref() {
                        cmd.arg(call_name);
                    }
                    cmd.arg(value);
                }
            }
            ArgKind::Path { value, default, .. } => match (&value[..], default, self.required) {
                ("", None, true) => return Err(()),
                ("", None, false) => {}
                ("", Some(default), _) => {
                    if let Some(call_name) = self.call_name.as_ref() {
                        cmd.arg(call_name);
                    }
                    cmd.arg(default);
                }
                (value, _, _) => {
                    if let Some(call_name) = self.call_name.as_ref() {
                        cmd.arg(call_name);
                    }
                    cmd.arg(value);
                }
            },
            ArgKind::MultiplePaths { values, .. } => {
                for value in values {
                    if let Some(call_name) = self.call_name.as_ref() {
                        cmd.arg(call_name);
                    }
                    cmd.arg(value);
                }
            }
        }

        Ok(cmd)
    }
}

impl From<&Arg<'_>> for ArgState {
    fn from(a: &Arg) -> Self {
        let call_name = a
            .get_long()
            .map(|s| format!("--{}", s))
            .or(a.get_short().map(|c| format!("-{}", c)));

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
            (false, true, ValueHint::AnyPath | ValueHint::DirPath | ValueHint::FilePath) => {
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
            (true, false, _) => ArgKind::Occurences(0),
            (false, false, _) => ArgKind::Bool(false),
        };

        Self {
            name: a.get_name().to_string(),
            call_name,
            desc,
            required: a.is_set(ArgSettings::Required),
            kind,
        }
    }
}
