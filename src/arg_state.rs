use crate::{error::ValidationErrorInfoTrait, KlaskUi, ValidationErrorInfo};
use clap::{Arg, ArgSettings, ValueHint};
use eframe::egui::{ComboBox, Ui};
use inflector::Inflector;
use native_dialog::FileDialog;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ArgState {
    pub name: String,
    pub call_name: Option<String>,
    pub desc: Option<String>,
    pub optional: bool,
    pub use_equals: bool,
    pub kind: ArgKind,
}

#[derive(Debug, Clone)]
pub enum ArgKind {
    String {
        value: String,
        default: Option<String>,
        possible: Vec<String>,
        value_hint: ValueHint,
        id: Uuid,
    },
    MultipleStrings {
        values: Vec<ChooseState>,
        default: Vec<ChooseState>,
        possible: Vec<String>,
        value_hint: ValueHint,
    },
    Occurences(i32),
    Bool(bool),
}

#[derive(Debug, Clone)]
pub struct ChooseState(pub String, pub Uuid);

impl Default for ChooseState {
    fn default() -> Self {
        Self(Default::default(), Uuid::new_v4())
    }
}

impl From<String> for ChooseState {
    fn from(s: String) -> Self {
        ChooseState(s, Uuid::new_v4())
    }
}

impl From<ChooseState> for String {
    fn from(s: ChooseState) -> Self {
        s.0
    }
}

impl From<&Arg<'_>> for ArgState {
    fn from(a: &Arg) -> Self {
        let kind = match (
            a.is_set(ArgSettings::MultipleOccurrences),
            a.is_set(ArgSettings::TakesValue),
        ) {
            (true, true) => ArgKind::MultipleStrings {
                values: vec![],
                default: a
                    .get_default_values()
                    .iter()
                    .map(|s| s.to_string_lossy().into_owned().into())
                    .collect(),
                possible: a
                    .get_possible_values()
                    .unwrap_or_default()
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                value_hint: a.get_value_hint(),
            },
            (false, true) => ArgKind::String {
                value: "".into(),
                default: a
                    .get_default_values()
                    .first()
                    .map(|s| s.to_string_lossy().into_owned()),
                possible: a
                    .get_possible_values()
                    .unwrap_or_default()
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                value_hint: a.get_value_hint(),
                id: Uuid::new_v4(),
            },
            (true, false) => ArgKind::Occurences(0),
            (false, false) => ArgKind::Bool(false),
        };

        Self {
            name: a.get_name().to_string().to_sentence_case(),
            call_name: a
                .get_long()
                .map(|s| format!("--{}", s))
                .or_else(|| a.get_short().map(|c| format!("-{}", c))),
            desc: a
                .get_long_about()
                .map(ToString::to_string)
                .or_else(|| a.get_about().map(ToString::to_string)),
            optional: !a.is_set(ArgSettings::Required),
            use_equals: a.is_set(ArgSettings::RequireEquals),
            kind,
        }
    }
}

impl ArgState {
    pub fn update(&mut self, ui: &mut Ui, validation_error: &mut Option<ValidationErrorInfo>) {
        let label = ui.label(&self.name);

        if let Some(desc) = &self.desc {
            label.on_hover_text(desc);
        }

        ui.horizontal(|ui| {
            // Not needed in edition 2021 with new closure borrowing rules
            let ArgState {
                name,
                optional,
                kind,
                ..
            } = self;

            match kind {
                ArgKind::String {
                    value,
                    default,
                    possible,
                    value_hint,
                    id,
                } => {
                    ui.error_style_if(
                        (!*optional && value.is_empty()) || validation_error.is(name).is_some(),
                        |ui| {
                            if possible.is_empty() {
                                if matches!(
                                    value_hint,
                                    ValueHint::AnyPath
                                        | ValueHint::FilePath
                                        | ValueHint::ExecutablePath
                                ) && ui.button("Select file...").clicked()
                                {
                                    if let Some(file) =
                                        FileDialog::new().show_open_single_file().ok().flatten()
                                    {
                                        *value = file.to_string_lossy().into_owned();
                                    }
                                }

                                if matches!(value_hint, ValueHint::AnyPath | ValueHint::DirPath)
                                    && ui.button("Select directory...").clicked()
                                {
                                    if let Some(file) =
                                        FileDialog::new().show_open_single_dir().ok().flatten()
                                    {
                                        *value = file.to_string_lossy().into_owned();
                                    }
                                }

                                let text = ui.text_edit_singleline_hint(
                                    value,
                                    match (default, *optional) {
                                        (Some(default), _) => default.as_str(),
                                        (_, true) => "(Optional)",
                                        (_, false) => "",
                                    },
                                );

                                if let Some(message) = validation_error.is(name) {
                                    if text.on_hover_text(message).changed() {
                                        *validation_error = None;
                                    }
                                }
                            } else {
                                ComboBox::from_id_source(id)
                                    .selected_text(value.clone())
                                    .show_ui(ui, |ui| {
                                        if *optional {
                                            ui.selectable_value(value, String::new(), "None");
                                        }
                                        for p in possible {
                                            ui.selectable_value(value, p.clone(), p);
                                        }
                                    });
                            }
                        },
                    );
                }
                ArgKind::Occurences(i) => {
                    let list = ui.horizontal(|ui| {
                        if ui.small_button("-").clicked() {
                            *i = (*i - 1).max(0);
                        }

                        ui.error_style_if(validation_error.is(name).is_some(), |ui| {
                            ui.label(i.to_string());
                        });

                        if ui.small_button("+").clicked() {
                            *i += 1;
                        }
                    });

                    if let Some(message) = validation_error.is(name) {
                        if list.response.on_hover_text(message).changed() {
                            *validation_error = None;
                        }
                    }
                }
                ArgKind::MultipleStrings {
                    values,
                    default,
                    possible,
                    value_hint,
                } => {
                    ui.multiple_values(
                        validation_error,
                        name,
                        values,
                        Some(default),
                        |ui, value| {
                            if possible.is_empty() {
                                if matches!(
                                    value_hint,
                                    ValueHint::AnyPath
                                        | ValueHint::FilePath
                                        | ValueHint::ExecutablePath
                                ) && ui.button("Select file...").clicked()
                                {
                                    if let Some(file) =
                                        FileDialog::new().show_open_single_file().ok().flatten()
                                    {
                                        value.0 = file.to_string_lossy().into_owned();
                                    }
                                }

                                if matches!(value_hint, ValueHint::AnyPath | ValueHint::DirPath)
                                    && ui.button("Select directory...").clicked()
                                {
                                    if let Some(file) =
                                        FileDialog::new().show_open_single_dir().ok().flatten()
                                    {
                                        value.0 = file.to_string_lossy().into_owned();
                                    }
                                }

                                ui.text_edit_singleline(&mut value.0);
                            } else {
                                let possible = possible.clone();
                                ComboBox::from_id_source(value.1)
                                    .selected_text(value.0.clone())
                                    .show_ui(ui, |ui| {
                                        if *optional {
                                            ui.selectable_value(
                                                &mut value.0,
                                                String::new(),
                                                "None",
                                            );
                                        }
                                        for s in possible {
                                            ui.selectable_value(&mut value.0, s.clone(), s);
                                        }
                                    });
                            }
                        },
                    );
                }
                ArgKind::Bool(bool) => {
                    ui.checkbox(bool, "");
                }
            };
        });
    }

    pub fn get_cmd_args(&self, mut args: Vec<String>) -> Result<Vec<String>, String> {
        match &self.kind {
            ArgKind::String { value, .. } => {
                if !value.is_empty() {
                    if let Some(call_name) = self.call_name.as_ref() {
                        if self.use_equals {
                            args.push(format!("{}={}", call_name, value));
                        } else {
                            args.extend_from_slice(&[call_name.clone(), value.clone()]);
                        }
                    } else {
                        args.push(value.clone());
                    }
                } else if !self.optional {
                    return Err(format!("{} is required.", self.name));
                }
            }
            &ArgKind::Occurences(i) => {
                for _ in 0..i {
                    args.push(
                        self.call_name
                            .clone()
                            .ok_or_else(|| "Internal error.".to_string())?,
                    );
                }
            }
            &ArgKind::Bool(bool) => {
                if bool {
                    args.push(
                        self.call_name
                            .clone()
                            .ok_or_else(|| "Internal error.".to_string())?,
                    );
                }
            }
            ArgKind::MultipleStrings { values, .. } => {
                for value in values {
                    if let Some(call_name) = self.call_name.as_ref() {
                        if self.use_equals {
                            args.push(format!("{}={}", call_name, value.0));
                        } else {
                            args.extend_from_slice(&[call_name.clone(), value.0.clone()]);
                        }
                    } else {
                        args.push(value.0.clone());
                    }
                }
            }
        }

        Ok(args)
    }
}
