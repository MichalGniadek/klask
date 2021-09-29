use crate::klask_ui::KlaskUi;
use eframe::egui::{ProgressBar, Ui};

/// Unicode non-character. Used for sending messages between GUI and user's program
const MAGIC: char = '\u{5FFFE}';
fn send_message(data: &[&str]) {
    for d in data {
        print!("{}{}", MAGIC, d);
    }
    println!("{}", MAGIC);
}

#[derive(Debug)]
pub struct Output(Vec<(u64, OutputType)>);

#[derive(Debug)]
pub enum OutputType {
    Text(String),
    ProgressBar(String, f32),
}

impl Output {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn parse(&mut self, str: &str) {
        let mut iter = str.split(MAGIC);

        if let Some(text) = iter.next() {
            if !text.is_empty() {
                self.0.push((0, OutputType::Text(text.to_string())))
            }
        }

        while let Some(id) = iter.next() {
            if let Ok(id) = id.parse() {
                if let Some(output) = OutputType::parse(&mut iter) {
                    if let Some((_, exists)) = self.0.iter_mut().find(|(i, _)| *i == id) {
                        *exists = output;
                    } else {
                        self.0.push((id, output));
                    }
                }

                if let Some(text) = iter.next() {
                    // Get rid of the newline
                    let text = &text[1..];
                    if !text.is_empty() {
                        self.0.push((0, OutputType::Text(text.to_string())))
                    }
                }
            }
        }
    }

    pub fn update(&mut self, ui: &mut Ui) {
        for (_, o) in &mut self.0 {
            match o {
                OutputType::Text(ref text) => ui.ansi_label(text),
                OutputType::ProgressBar(ref mess, value) => {
                    ui.add(ProgressBar::new(*value).text(mess).animate(true));
                }
            }
        }
    }
}

impl OutputType {
    const PROGRESS_BAR_STR: &'static str = "progress-bar";

    pub fn send(self, id: u64) {
        match self {
            OutputType::Text(s) => print!("{}", s),
            OutputType::ProgressBar(desc, value) => send_message(&[
                &id.to_string(),
                Self::PROGRESS_BAR_STR,
                &desc,
                &value.to_string(),
            ]),
        }
    }

    pub fn parse<'a>(iter: &mut impl Iterator<Item = &'a str>) -> Option<Self> {
        match iter.next() {
            Some(Self::PROGRESS_BAR_STR) => Some(Self::ProgressBar(
                iter.next().unwrap_or_default().to_string(),
                iter.next()
                    .map(|s| s.parse().ok())
                    .flatten()
                    .unwrap_or_default(),
            )),
            None => None,
            _ => panic!(),
        }
    }
}
