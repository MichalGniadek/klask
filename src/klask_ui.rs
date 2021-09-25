use crate::error::{ValidationErrorInfo, ValidationErrorInfoTrait};
use cansi::{CategorisedSlice, Color};
use eframe::egui::{Color32, Label, Response, TextEdit, Ui};
use linkify::{LinkFinder, LinkKind};

pub trait KlaskUi {
    fn error_style_if<F: FnOnce(&mut Ui) -> R, R>(&mut self, error: bool, f: F) -> R;
    fn text_edit_singleline_hint(&mut self, text: &mut String, hint: impl ToString) -> Response;
    fn ansi_label(&mut self, text: &str);
    fn multiple_values<T, F>(
        &mut self,
        validation_error: &mut Option<ValidationErrorInfo>,
        name: &str,
        values: &mut Vec<T>,
        default: Option<&mut Vec<T>>,
        f: F,
    ) where
        T: Clone + Default,
        F: FnMut(&mut Ui, &mut T);
}

impl KlaskUi for Ui {
    fn error_style_if<F: FnOnce(&mut Ui) -> R, R>(&mut self, is_error: bool, f: F) -> R {
        let previous = if is_error {
            let visuals = &mut self.style_mut().visuals;
            let previous = visuals.clone();
            visuals.widgets.inactive.bg_stroke.color = Color32::RED;
            visuals.widgets.inactive.bg_stroke.width = 1.0;
            visuals.widgets.hovered.bg_stroke.color = Color32::RED;
            visuals.widgets.active.bg_stroke.color = Color32::RED;
            visuals.widgets.open.bg_stroke.color = Color32::RED;
            visuals.widgets.noninteractive.bg_stroke.color = Color32::RED;
            visuals.selection.stroke.color = Color32::RED;
            Some(previous)
        } else {
            None
        };

        let ret = f(self);

        if let Some(previous) = previous {
            self.style_mut().visuals = previous;
        }

        ret
    }

    fn text_edit_singleline_hint(&mut self, text: &mut String, hint: impl ToString) -> Response {
        self.add(TextEdit::singleline(text).hint_text(hint))
    }

    fn ansi_label(&mut self, text: &str) {
        let output = cansi::categorise_text(text);

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
                    Some(LinkKind::Url) => self.hyperlink(span.as_str()),
                    Some(LinkKind::Email) => self.hyperlink(format!("mailto:{}", span.as_str())),
                    Some(_) | None => {
                        let mut label =
                            Label::new(span.as_str()).text_color(ansi_color_to_egui(fg_colour));

                        if bg_colour != Color::Black {
                            label = label.background_color(ansi_color_to_egui(bg_colour));
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

                        self.add(label)
                    }
                };
            }
        }
    }

    fn multiple_values<T, F>(
        &mut self,
        validation_error: &mut Option<ValidationErrorInfo>,
        name: &str,
        values: &mut Vec<T>,
        default: Option<&mut Vec<T>>,
        mut f: F,
    ) where
        T: Clone + Default,
        F: FnMut(&mut Ui, &mut T),
    {
        let list = self.vertical(|ui| {
            ui.error_style_if(validation_error.is(name).is_some(), |ui| {
                let mut remove_index = None;

                for (index, value) in values.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        if ui.small_button("-").clicked() {
                            remove_index = Some(index);
                        }

                        f(ui, value);
                    });
                }

                if let Some(index) = remove_index {
                    values.remove(index);
                }

                ui.horizontal(|ui| {
                    if ui.button("New value").clicked() {
                        values.push(T::default());
                    }
                    if let Some(default) = default {
                        let text = if default.is_empty() {
                            "Reset"
                        } else {
                            "Reset to default"
                        };
                        ui.add_space(20.0);
                        if ui.button(text).clicked() {
                            *values = default.clone();
                        }
                    }
                });
            })
        });

        if let Some(message) = validation_error.is(name) {
            if list.response.on_hover_text(message).changed() {
                *validation_error = None;
            }
        }
    }
}

fn ansi_color_to_egui(color: Color) -> Color32 {
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
