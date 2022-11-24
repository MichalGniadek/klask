use crate::child_app::ChildApp;
use crate::error::ExecutionError;
use cansi::{v3::CategorisedSlice, Color, Intensity};
use eframe::egui::{vec2, Color32, Label, ProgressBar, RichText, Ui, Widget};
use linkify::{LinkFinder, LinkKind};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Write;

/// Displays a progress bar in the output. First call creates
/// a progress bar and future calls update it.
///
/// Value is a f32 between 0 and 1.
///
/// If the description is not static or you need to use the same description
/// multiple times, use [`progress_bar_with_id`].
/// ```no_run
/// # use clap::{App, Arg};
/// # use klask::Settings;
/// fn main() {
///     klask::run_app(App::new("Example"), Settings::default(), |matches| {
///         for i in 0..=100 {
///             klask::output::progress_bar("Static description", i as f32 / 100.0);
///         }
///     });
/// }
/// ```
pub fn progress_bar(description: &str, value: f32) {
    progress_bar_with_id(description, description, value);
}

/// Displays a progress bar in the output. First call creates
/// a progress bar and future calls update it.
///
/// Value is a f32 between 0 and 1.
/// Id is any hashable value that uniquely identifies a progress bar.
/// ```no_run
/// # use clap::{App, Arg};
/// # use klask::Settings;
/// fn main() {
///     klask::run_app(App::new("Example"), Settings::default(), |matches| {
///         for i in 0..=100 {
///             klask::output::progress_bar_with_id(
///                 "Progress",
///                 &format!("Dynamic description [{}/{}]", i, 100),
///                 i as f32 / 100.0,
///             );
///         }
///     });
/// }
/// ```
pub fn progress_bar_with_id(id: impl Hash, description: &str, value: f32) {
    let mut h = DefaultHasher::new();
    id.hash(&mut h);
    OutputType::ProgressBar(description.to_string(), value).send(h.finish());
}

#[derive(Debug)]
pub(crate) enum Output {
    None,
    Err(ExecutionError),
    Child(ChildApp, Vec<(u64, OutputType)>),
}

impl Output {
    pub fn new_with_child(child: ChildApp) -> Self {
        Self::Child(child, vec![])
    }
}

impl Widget for &mut Output {
    fn ui(self, ui: &mut Ui) -> eframe::egui::Response {
        match self {
            Output::None => ui.vertical(|_| {}).response,
            Output::Err(err) => ui.colored_label(Color32::RED, err.to_string()),
            Output::Child(child, output) => {
                // Update
                let str = child.read();
                let mut iter = str.split(MAGIC);

                if let Some(text) = iter.next() {
                    if !text.is_empty() {
                        output.push((0, OutputType::Text(text.to_string())));
                    }
                }

                while let Some(id) = iter.next() {
                    if let Ok(id) = id.parse() {
                        if let Some(new) = OutputType::parse(&mut iter) {
                            if let Some((_, exists)) = output.iter_mut().find(|(i, _)| *i == id) {
                                *exists = new;
                            } else {
                                output.push((id, new));
                            }
                        }
                    }

                    if let Some(text) = iter.next() {
                        // Get rid of the newline
                        let text = &text[1..];
                        if !text.is_empty() {
                            output.push((0, OutputType::Text(text.to_string())));
                        }
                    }
                }

                // View
                ui.vertical(|ui| {
                    if ui.button("Copy output").clicked() {
                        ui.ctx().output().copied_text = output
                            .iter()
                            .map(|(_, o)| match o {
                                OutputType::Text(text) => text,
                                OutputType::ProgressBar(text, _) => text,
                            })
                            .flat_map(|text| cansi::v3::categorise_text(text))
                            .map(|slice| slice.text)
                            .collect::<String>();
                    }

                    for (_, o) in output {
                        match o {
                            OutputType::Text(ref text) => format_output(ui, text),
                            OutputType::ProgressBar(ref mess, value) => {
                                // Get rid of the ending newline
                                ui.add(
                                    ProgressBar::new(*value)
                                        .text(&mess[..mess.len() - 1])
                                        .animate(true),
                                );
                            }
                        }
                    }
                })
                .response
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum OutputType {
    Text(String),
    ProgressBar(String, f32),
}

/// Unicode non-character. Used for sending messages between GUI and user's program
const MAGIC: char = '\u{5FFFE}';

fn send_message(data: &[&str]) {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    for d in data {
        write!(&mut lock, "{}{}", MAGIC, d).unwrap();
    }
    writeln!(&mut lock, "{}", MAGIC).unwrap();
}

impl OutputType {
    const PROGRESS_BAR_STR: &'static str = "progress-bar";

    pub fn send(self, id: u64) {
        // Make sure to get rid of any newlines
        match self {
            Self::Text(s) => print!("{}", s),
            Self::ProgressBar(desc, value) => send_message(&[
                &id.to_string(),
                Self::PROGRESS_BAR_STR,
                &desc.replace('\n', " "),
                &value.to_string(),
            ]),
        }
    }

    pub fn parse<'a>(iter: &mut impl Iterator<Item = &'a str>) -> Option<Self> {
        match iter.next() {
            // Add a newline here for copying out text
            Some(Self::PROGRESS_BAR_STR) => Some(Self::ProgressBar(
                format!("{}\n", iter.next().unwrap_or_default()),
                iter.next().and_then(|s| s.parse().ok()).unwrap_or_default(),
            )),
            _ => None,
        }
    }
}

fn format_output(ui: &mut Ui, text: &str) {
    let output = cansi::v3::categorise_text(text);

    let previous = ui.style().spacing.item_spacing;
    ui.style_mut().spacing.item_spacing = vec2(0.0, 0.0);

    ui.horizontal_wrapped(|ui| {
        for CategorisedSlice {
            text,
            fg,
            bg,
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
                        ui.hyperlink_to(span.as_str(), format!("mailto:{}", span.as_str()))
                    }
                    Some(_) | None => {
                        let mut text = RichText::new(span.as_str());

                        if let Some(fg) = fg {
                            text = text.color(ansi_color_to_egui(fg));
                        }

                        if let Some(bg) = bg {
                            if bg != Color::Black {
                                text = text.background_color(ansi_color_to_egui(bg));
                            }
                        }

                        if italic == Some(true) {
                            text = text.italics();
                        }

                        if underline == Some(true) {
                            text = text.underline();
                        }

                        if strikethrough == Some(true) {
                            text = text.strikethrough();
                        }

                        text = match intensity {
                            Some(Intensity::Bold) => text.strong(),
                            Some(Intensity::Faint) => text.weak(),
                            Some(Intensity::Normal) | None => text,
                        };

                        ui.add(Label::new(text))
                    }
                };
            }
        }
    });
    ui.style_mut().spacing.item_spacing = previous;
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
