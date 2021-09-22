use crate::MyUi;
use clap::{Arg, ArgSettings, ValueHint};
use eframe::egui::{ComboBox, TextEdit, Ui};
use inflector::Inflector;
use native_dialog::FileDialog;
use std::process::Command;
use uuid::Uuid;

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
    Choose {
        value: (String, Uuid),
        possible: Vec<String>,
    },
    MultipleChoose {
        values: Vec<(String, Uuid)>,
        possible: Vec<String>,
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
                    let required = self.required;
                    ui.error_style_if(self.required && value.is_empty(), |ui| {
                        ui.add(
                            TextEdit::singleline(value)
                                .hint_text(default.clone().unwrap_or(if required {
                                    String::new()
                                } else {
                                    String::from("(Optional)")
                                }))
                                .desired_width(f32::MAX),
                        );
                    });
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
                        if let Some(file) = FileDialog::new().show_open_single_file().ok().flatten()
                        {
                            *value = file.to_string_lossy().into_owned();
                        }
                    }

                    if *allow_dir && ui.button("Select directory...").clicked() {
                        if let Some(file) = FileDialog::new().show_open_single_dir().ok().flatten()
                        {
                            *value = file.to_string_lossy().into_owned();
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
                                    if let Some(file) =
                                        FileDialog::new().show_open_single_file().ok().flatten()
                                    {
                                        *value = file.to_string_lossy().into_owned();
                                    }
                                }

                                if *allow_dir && ui.button("Select directory...").clicked() {
                                    if let Some(file) =
                                        FileDialog::new().show_open_single_dir().ok().flatten()
                                    {
                                        *value = file.to_string_lossy().into_owned();
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
                ArgKind::Choose {
                    value: (value, id),
                    possible,
                } => {
                    let required = self.required;
                    ComboBox::from_id_source(id)
                        .selected_text(value.clone())
                        .show_ui(ui, |ui| {
                            if !required {
                                ui.selectable_value(value, String::new(), "None");
                            }
                            for p in possible {
                                ui.selectable_value(value, p.clone(), p);
                            }
                        });
                }
                ArgKind::MultipleChoose {
                    values,
                    ref possible,
                } => {
                    ui.vertical(|ui| {
                        let mut remove_index = None;
                        for (index, (value, id)) in values.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                if ui.small_button("-").clicked() {
                                    remove_index = Some(index);
                                }
                                ComboBox::from_id_source(id)
                                    .selected_text(value.clone())
                                    .show_ui(ui, |ui| {
                                        for p in possible {
                                            ui.selectable_value(value, p.clone(), p);
                                        }
                                    });
                            });
                        }

                        if let Some(index) = remove_index {
                            values.remove(index);
                        }

                        if ui.button("New value").clicked() {
                            values.push((String::new(), Uuid::new_v4()));
                        }
                    });
                }
            };
        });
    }

    pub fn cmd_args(&self, mut cmd: Command) -> Result<Command, String> {
        match &self.kind {
            ArgKind::String { value, default } => {
                match (&value[..], default, self.required) {
                    ("", None, true) => return Err(format!("{} is required.", self.name)),
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
                    cmd.arg(
                        self.call_name
                            .as_ref()
                            .ok_or_else(|| format!("Internal error."))?,
                    );
                }
            }
            &ArgKind::Bool(bool) => {
                if bool {
                    cmd.arg(
                        self.call_name
                            .as_ref()
                            .ok_or_else(|| format!("Internal error."))?,
                    );
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
                ("", None, true) => return Err(format!("{} is required.", self.name)),
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
            ArgKind::Choose {
                value: (value, _), ..
            } => {
                if let Some(call_name) = self.call_name.as_ref() {
                    cmd.arg(call_name);
                }
                cmd.arg(value);
            }
            ArgKind::MultipleChoose { values, .. } => {
                for (value, _) in values {
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
        let mut call_name = a
            .get_long()
            .map(|s| format!("--{}", s))
            .or_else(|| a.get_short().map(|c| format!("-{}", c)));

        if a.is_set(ArgSettings::RequireEquals) {
            if let Some(call_name) = &mut call_name {
                call_name.push('=');
            }
        }

        let desc = a
            .get_long_about()
            .map(ToString::to_string)
            .or_else(|| a.get_about().map(ToString::to_string));

        let required = a.is_set(ArgSettings::Required) | a.is_set(ArgSettings::ForbidEmptyValues);

        use ValueHint::*;
        let kind = match (
            a.is_set(ArgSettings::MultipleOccurrences),
            a.is_set(ArgSettings::TakesValue),
            a.get_value_hint(),
            a.get_possible_values(),
        ) {
            (true, true, AnyPath | DirPath | FilePath | ExecutablePath, None) => {
                let default: Vec<_> = a
                    .get_default_values()
                    .iter()
                    .map(|s| s.to_string_lossy().into_owned())
                    .collect();

                ArgKind::MultiplePaths {
                    values: default.clone(),
                    default,
                    allow_dir: matches!(a.get_value_hint(), AnyPath | DirPath),
                    allow_file: matches!(a.get_value_hint(), AnyPath | FilePath | ExecutablePath),
                }
            }
            (true, true, _, None) => {
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
            (false, true, AnyPath | DirPath | FilePath | ExecutablePath, None) => {
                let default = a
                    .get_default_values()
                    .first()
                    .map(|s| s.to_string_lossy().into_owned());

                ArgKind::Path {
                    value: default.clone().unwrap_or_default(),
                    default,
                    allow_dir: matches!(a.get_value_hint(), AnyPath | DirPath),
                    allow_file: matches!(a.get_value_hint(), AnyPath | FilePath | ExecutablePath),
                }
            }
            (false, true, _, None) => ArgKind::String {
                value: "".into(),
                default: a
                    .get_default_values()
                    .first()
                    .map(|s| s.to_string_lossy().into_owned()),
            },
            (true, false, _, None) => ArgKind::Occurences(0),
            (false, false, _, None) => ArgKind::Bool(false),
            (false, _, _, Some(possible)) => ArgKind::Choose {
                value: (
                    if required {
                        possible[0].to_string()
                    } else {
                        "".into()
                    },
                    Uuid::new_v4(),
                ),
                possible: possible.iter().map(|s| s.to_string()).collect(),
            },
            (true, _, _, Some(possible)) => ArgKind::MultipleChoose {
                values: vec![],
                possible: possible.iter().map(|s| s.to_string()).collect(),
            },
        };

        Self {
            name: a.get_name().to_string().to_sentence_case(),
            call_name,
            desc,
            required,
            kind,
        }
    }
}
