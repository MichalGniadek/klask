use crate::{settings::Localization, Klask};
use clap::{Arg, ValueHint};
use eframe::egui::{widgets::Widget, ComboBox, Response, TextEdit, Ui};
use inflector::Inflector;
use rfd::FileDialog;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ArgState<'s> {
    pub name: String,
    pub call_name: Option<String>,
    pub desc: Option<String>,
    pub optional: bool,
    pub use_equals: bool,
    pub forbid_empty: bool,
    pub kind: ArgKind,
    pub validation_error: Option<String>,
    pub localization: &'s Localization,
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
        multiple_values: bool,
        multiple_occurrences: bool,
        use_delimiter: bool,
        req_delimiter: bool,
        value_hint: ValueHint,
    },
    Occurences(i32),
    Bool(bool),
}

impl<'s> ArgState<'s> {
    pub fn new(arg: &Arg, localization: &'s Localization) -> Self {
        let kind = if arg.is_takes_value_set() {
            let mut default = arg
                .get_default_values()
                .iter()
                .map(|s| s.to_string_lossy().into_owned());

            let possible = arg
                .get_possible_values()
                .unwrap_or_default()
                .iter()
                .map(|v| v.get_name().to_string())
                .collect();

            let multiple_values = arg.is_multiple_values_set();
            let multiple_occurrences = arg.is_multiple_occurrences_set();

            if multiple_occurrences | multiple_values {
                ArgKind::MultipleStrings {
                    values: vec![],
                    default: default.collect(),
                    possible,
                    multiple_values,
                    multiple_occurrences,
                    use_delimiter: arg.is_use_value_delimiter_set()
                        | arg.is_require_value_delimiter_set(),
                    req_delimiter: arg.is_require_value_delimiter_set(),
                    value_hint: arg.get_value_hint(),
                }
            } else {
                ArgKind::String {
                    value: (String::new(), Uuid::new_v4()),
                    default: default.next(),
                    possible,
                    value_hint: arg.get_value_hint(),
                }
            }
        } else if arg.is_multiple_occurrences_set() {
            ArgKind::Occurences(0)
        } else {
            ArgKind::Bool(false)
        };

        Self {
            name: arg.get_id().to_string().to_sentence_case(),
            call_name: arg
                .get_long()
                .map(|s| format!("--{}", s))
                .or_else(|| arg.get_short().map(|c| format!("-{}", c))),
            desc: arg
                .get_long_help()
                .map(ToString::to_string)
                .or_else(|| arg.get_help().map(ToString::to_string)),
            optional: !arg.is_required_set(),
            use_equals: arg.is_require_equals_set(),
            forbid_empty: arg.is_forbid_empty_values_set(),
            kind,
            validation_error: None,
            localization,
        }
    }

    pub fn update_validation_error(&mut self, name: &str, message: &str) {
        self.validation_error = (self.name == name).then(|| message.to_string());
    }

    #[allow(clippy::too_many_arguments)]
    pub fn ui_single_row(
        ui: &mut Ui,
        (value, id): &mut (String, Uuid),
        default: &Option<String>,
        possible: &[String],
        value_hint: ValueHint,
        optional: bool,
        validation_error: bool,
        localization: &'s Localization,
    ) -> Response {
        let is_error = (!optional && value.is_empty()) || validation_error;
        if is_error {
            Klask::set_error_style(ui);
        }

        let inner_response = if possible.is_empty() {
            ui.horizontal(|ui| {
                if matches!(
                    value_hint,
                    ValueHint::AnyPath | ValueHint::FilePath | ValueHint::ExecutablePath
                ) && ui.button(&localization.select_file).clicked()
                {
                    if let Some(file) = FileDialog::new().pick_file() {
                        *value = file.to_string_lossy().into_owned();
                    }
                }

                if matches!(value_hint, ValueHint::AnyPath | ValueHint::DirPath)
                    && ui.button(&localization.select_directory).clicked()
                {
                    if let Some(file) = FileDialog::new().pick_folder() {
                        *value = file.to_string_lossy().into_owned();
                    }
                }

                ui.add(
                    TextEdit::singleline(value).hint_text(match (default, optional) {
                        (Some(default), _) => default.as_str(),
                        (_, true) => localization.optional.as_str(),
                        (_, false) => "",
                    }),
                );

                Some(())
            })
        } else {
            ComboBox::from_id_source(id)
                .selected_text(&*value)
                .show_ui(ui, |ui| {
                    if optional {
                        ui.selectable_value(value, String::new(), "None");
                    }
                    for p in possible {
                        ui.selectable_value(value, p.clone(), p);
                    }
                })
        };

        if is_error {
            ui.reset_style();
        }

        inner_response.response
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
                    return Err(format!(
                        "{}{}{}",
                        self.localization.error_is_required.0,
                        self.name,
                        self.localization.error_is_required.1
                    ));
                }
            }
            ArgKind::MultipleStrings {
                values,
                multiple_values,
                multiple_occurrences,
                use_delimiter,
                req_delimiter,
                ..
            } => {
                if !values.is_empty() {
                    if let Some(call_name) = &self.call_name {
                        let single = *use_delimiter || values.len() == 1;
                        match (
                            self.use_equals,
                            *multiple_values,
                            *multiple_occurrences,
                            single,
                        ) {
                            (true, true, _, true) => {
                                args.push(format!(
                                    "{}={}",
                                    call_name,
                                    &values
                                        .iter()
                                        .map(|(s, _)| format!(",{}", s))
                                        .collect::<String>()[1..]
                                ));
                            }
                            (false, true, _, _) => {
                                args.push(call_name.clone());

                                if *req_delimiter {
                                    args.push(
                                        (values
                                            .iter()
                                            .map(|(s, _)| format!(",{}", s))
                                            .collect::<String>()[1..])
                                            .to_string(),
                                    );
                                } else {
                                    for value in values {
                                        args.push(value.0.clone());
                                    }
                                }
                            }
                            (true, _, true, _) => {
                                for value in values {
                                    args.push(format!("{}={}", call_name, value.0));
                                }
                            }
                            (false, _, true, _) => {
                                for value in values {
                                    args.extend_from_slice(&[call_name.clone(), value.0.clone()]);
                                }
                            }
                            (_, false, false, _) => unreachable!(
                                "Either multiple_values or multiple_occurrences must be true"
                            ),
                            (true, true, false, false) => return Err("Can't be represented".into()),
                        }
                    } else {
                        for value in values {
                            args.push(value.0.clone());
                        }
                    }
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
        }

        Ok(args)
    }
}

impl Widget for &mut ArgState<'_> {
    fn ui(self, ui: &mut Ui) -> eframe::egui::Response {
        let localization = self.localization;
        let label = ui.label(&self.name);

        if let Some(desc) = &self.desc {
            label.on_hover_text(desc);
        }

        // Grid column automatically switches here

        let is_validation_error = self.validation_error.is_some();

        match &mut self.kind {
            ArgKind::String {
                value,
                default,
                possible,
                value_hint,
            } => ArgState::ui_single_row(
                ui,
                value,
                default,
                possible,
                *value_hint,
                self.optional && !self.forbid_empty,
                is_validation_error,
                localization,
            ),
            ArgKind::MultipleStrings {
                values,
                default,
                possible,
                value_hint,
                ..
            } => {
                let forbid_empty = self.forbid_empty;
                let mut list = ui
                    .vertical(|ui| {
                        let mut remove_index = None;

                        for (index, value) in values.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                if ui.small_button("-").clicked() {
                                    remove_index = Some(index);
                                }

                                ArgState::ui_single_row(
                                    ui,
                                    value,
                                    &None,
                                    possible,
                                    *value_hint,
                                    !forbid_empty,
                                    is_validation_error,
                                    localization,
                                );
                            });
                        }

                        if let Some(index) = remove_index {
                            values.remove(index);
                        }

                        ui.horizontal(|ui| {
                            if ui.button(&localization.new_value).clicked() {
                                values.push((String::new(), Uuid::new_v4()));
                            }

                            let text = if default.is_empty() {
                                &localization.reset
                            } else {
                                &localization.reset_to_default
                            };

                            ui.add_space(20.0);
                            if ui.button(text).clicked() {
                                *values = default
                                    .iter()
                                    .map(|s| (s.to_string(), Uuid::new_v4()))
                                    .collect();
                            }
                        });
                    })
                    .response;

                if let Some(message) = &self.validation_error {
                    list = list.on_hover_text(message);
                    if list.changed() {
                        self.validation_error = None;
                    }
                }

                list
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
                })
                .response
            }
            ArgKind::Bool(bool) => ui.checkbox(bool, ""),
        }
    }
}
