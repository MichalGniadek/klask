use crate::{error::ValidationErrorInfoTrait, klask_ui, KlaskUi, ValidationErrorInfo};
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
    pub forbid_empty: bool,
    pub kind: ArgKind,
}

#[derive(Debug, Clone)]
pub enum ArgKind {
    String {
        value: (String, Uuid),
        default: Option<String>,
        possible: Vec<String>,
        value_hint: ValueHint,
    },
    MultipleStrings {
        values: Vec<(String, Uuid)>,
        default: Vec<String>,
        possible: Vec<String>,
        value_hint: ValueHint,
    },
    Occurences(i32),
    Bool(bool),
}

impl From<&Arg<'_>> for ArgState {
    fn from(a: &Arg) -> Self {
        let kind = if a.is_set(ArgSettings::TakesValue) {
            let mut default = a
                .get_default_values()
                .iter()
                .map(|s| s.to_string_lossy().into_owned());

            let possible = a
                .get_possible_values()
                .unwrap_or_default()
                .iter()
                .map(|s| s.to_string())
                .collect();

            if a.is_set(ArgSettings::MultipleOccurrences) {
                ArgKind::MultipleStrings {
                    values: vec![],
                    default: default.collect(),
                    possible,
                    value_hint: a.get_value_hint(),
                }
            } else {
                ArgKind::String {
                    value: ("".to_string(), Uuid::new_v4()),
                    default: default.next(),
                    possible,
                    value_hint: a.get_value_hint(),
                }
            }
        } else if a.is_set(ArgSettings::MultipleOccurrences) {
            ArgKind::Occurences(0)
        } else {
            ArgKind::Bool(false)
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
            forbid_empty: a.is_set(ArgSettings::ForbidEmptyValues),
            kind,
        }
    }
}

impl ArgState {
    pub fn update_single(
        ui: &mut Ui,
        (value, id): &mut (String, Uuid),
        default: &Option<String>,
        possible: &[String],
        value_hint: ValueHint,
        optional: bool,
        validation_error: bool,
    ) {
        let is_error = (!optional && value.is_empty()) || validation_error;
        let previous_style = is_error.then(|| klask_ui::set_error_style(ui));

        if possible.is_empty() {
            ui.horizontal(|ui| {
                if matches!(
                    value_hint,
                    ValueHint::AnyPath | ValueHint::FilePath | ValueHint::ExecutablePath
                ) && ui.button("Select file...").clicked()
                {
                    if let Some(file) = FileDialog::new().show_open_single_file().ok().flatten() {
                        *value = file.to_string_lossy().into_owned();
                    }
                }

                if matches!(value_hint, ValueHint::AnyPath | ValueHint::DirPath)
                    && ui.button("Select directory...").clicked()
                {
                    if let Some(file) = FileDialog::new().show_open_single_dir().ok().flatten() {
                        *value = file.to_string_lossy().into_owned();
                    }
                }

                ui.text_edit_singleline_hint(
                    value,
                    match (default, optional) {
                        (Some(default), _) => default.as_str(),
                        (_, true) => "(Optional)",
                        (_, false) => "",
                    },
                );
            });
        } else {
            ComboBox::from_id_source(id)
                .selected_text(&value)
                .show_ui(ui, |ui| {
                    if optional {
                        ui.selectable_value(value, String::new(), "None");
                    }
                    for p in possible {
                        ui.selectable_value(value, p.clone(), p);
                    }
                });
        }

        if let Some(previous) = previous_style {
            ui.set_style(previous);
        }
    }

    pub fn update(&mut self, ui: &mut Ui, validation_error: &mut Option<ValidationErrorInfo>) {
        let label = ui.label(&self.name);

        if let Some(desc) = &self.desc {
            label.on_hover_text(desc);
        }

        let is_validation_error = validation_error.is(&self.name).is_some();

        match &mut self.kind {
            ArgKind::String {
                value,
                default,
                possible,
                value_hint,
            } => Self::update_single(
                ui,
                value,
                default,
                possible,
                *value_hint,
                self.optional && !self.forbid_empty,
                is_validation_error,
            ),
            ArgKind::MultipleStrings {
                values,
                default,
                possible,
                value_hint,
            } => {
                let forbid_entry = self.forbid_empty;
                let list = ui.vertical(|ui| {
                    let mut remove_index = None;

                    for (index, value) in values.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            if ui.small_button("-").clicked() {
                                remove_index = Some(index);
                            }

                            Self::update_single(
                                ui,
                                value,
                                &None,
                                possible,
                                *value_hint,
                                !forbid_entry,
                                is_validation_error,
                            );
                        });
                    }

                    if let Some(index) = remove_index {
                        values.remove(index);
                    }

                    ui.horizontal(|ui| {
                        if ui.button("New value").clicked() {
                            values.push(("".into(), Uuid::new_v4()));
                        }

                        let text = if default.is_empty() {
                            "Reset"
                        } else {
                            "Reset to default"
                        };

                        ui.add_space(20.0);
                        if ui.button(text).clicked() {
                            *values = default
                                .iter()
                                .map(|s| (s.to_string(), Uuid::new_v4()))
                                .collect();
                        }
                    });
                });

                if let Some(message) = validation_error.is(&self.name) {
                    if list.response.on_hover_text(message).changed() {
                        *validation_error = None;
                    }
                }
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
    }

    pub fn get_cmd_args(&self, mut args: Vec<String>) -> Result<Vec<String>, String> {
        match &self.kind {
            ArgKind::String {
                value: (value, _), ..
            } => {
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
