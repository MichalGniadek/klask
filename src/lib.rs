#![feature(command_access)] // only for debugging
mod app_state;
mod arg_state;

use app_state::AppState;
use cansi::{CategorisedSlice, Color};
use clap::{App, ArgMatches, FromArgMatches, IntoApp};
use eframe::{
    egui::{self, Button, Color32, Label, Ui},
    epi,
};
use linkify::{LinkFinder, LinkKind};
use std::{
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

    pub fn run_derived<C, F>(f: F)
    where
        C: IntoApp + FromArgMatches,
        F: FnOnce(C),
    {
        Self::run_app(C::into_app(), |m| f(C::from_arg_matches(m).unwrap()));
    }

    fn run_command(&mut self) {
        let mut cmd = Command::new(std::env::current_exe().unwrap());
        cmd.stdout(Stdio::piped()).arg(&self.name);
        match self.state.cmd_args(cmd) {
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

    fn update_output(&self, ui: &mut Ui) {
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
                    Some(LinkKind::Email) => ui.hyperlink(format!("mailto:{}", span.as_str())),
                    Some(_) | None => {
                        let mut label =
                            Label::new(span.as_str()).text_color(convert_color(fg_colour));

                        if bg_colour != Color::Black {
                            label = label.background_color(convert_color(bg_colour));
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
                        self.run_command();
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

                self.update_output(ui);
            });
        });
    }

    fn on_exit(&mut self) {
        if let Some((child, _)) = &mut self.child {
            let _ = child.kill();
            self.child = None;
        }
    }
}

fn convert_color(color: Color) -> Color32 {
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
