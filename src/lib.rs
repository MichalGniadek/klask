#![feature(command_access)]
#![deny(clippy::unwrap_used, clippy::expect_used)]
mod app_state;
mod arg_state;

use app_state::AppState;
use cansi::{CategorisedSlice, Color};
use clap::{App, ArgMatches, FromArgMatches, IntoApp};
use eframe::{
    egui::{self, Button, Color32, Label, Response, TextEdit, Ui},
    epi,
};
use linkify::{LinkFinder, LinkKind};
use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread::{self},
};

pub struct ChildApp {
    child: Child,
    stdout: Option<Receiver<Option<String>>>,
    stderr: Option<Receiver<Option<String>>>,
}

pub struct Klask {
    name: String,
    child: Option<ChildApp>,
    output: Result<String, String>,
    state: AppState,
    // This isn't a generic lifetime because eframe::run_native() requires
    // a 'static lifetime because boxed trait objects default to 'static
    app: App<'static>,
}

impl Klask {
    pub fn run_app(app: App<'static>, f: impl FnOnce(&ArgMatches)) {
        match App::new("Outer GUI")
            .subcommand(app.clone())
            .try_get_matches()
        {
            Ok(matches) => match matches.subcommand_matches(app.get_name()) {
                Some(m) => f(m),
                None => {
                    let klask = Self {
                        name: app.get_name().to_string(),
                        child: None,
                        output: Ok(String::new()),
                        state: AppState::new(&app),
                        app,
                    };
                    let native_options = eframe::NativeOptions::default();
                    eframe::run_native(Box::new(klask), native_options);
                }
            },
            Err(err) => panic!(
                "Internal error, arguments should've been verified by the GUI app {:#?}",
                err
            ),
        }
    }

    pub fn run_derived<C, F>(f: F)
    where
        C: IntoApp + FromArgMatches,
        F: FnOnce(C),
    {
        Self::run_app(C::into_app(), |m| match C::from_arg_matches(m) {
            Some(c) => f(c),
            None => panic!("Internal error, C::from_arg_matches should always succeed"),
        });
    }

    fn execute_command(&mut self) -> Result<String, String> {
        let mut cmd = Command::new(
            std::env::current_exe().map_err(|_| String::from("Couldn't get current exe"))?,
        );
        cmd.arg(&self.name)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        match self.state.cmd_args(cmd) {
            Ok(mut cmd) => {
                if let Err(err) = self.app.clone().try_get_matches_from(cmd.get_args()) {
                    return Err(format!("Match error: {:?}  {:?}", err.kind, err.info));
                }

                let mut child = cmd
                    .spawn()
                    .map_err(|_| String::from("Couldn't spawn a child"))?;

                let mut stdout_reader = BufReader::new(
                    child
                        .stdout
                        .take()
                        .ok_or_else(|| String::from("Couldn't take stdout"))?,
                );
                let (stdout_tx, stdout_rx) = mpsc::channel();
                thread::spawn(move || loop {
                    let mut output = String::new();
                    if let Ok(0) = stdout_reader.read_line(&mut output) {
                        // End of output
                        let _ = stdout_tx.send(None);
                        break;
                    }
                    if stdout_tx.send(Some(output)).is_err() {
                        // Send returns error only if data will never be received
                        break;
                    }
                });

                let mut stderr_reader = BufReader::new(
                    child
                        .stderr
                        .take()
                        .ok_or_else(|| String::from("Couldn't take stderr"))?,
                );
                let (stderr_tx, stderr_rx) = mpsc::channel();
                thread::spawn(move || loop {
                    let mut output = String::new();
                    if let Ok(0) = stderr_reader.read_line(&mut output) {
                        // End of output
                        let _ = stderr_tx.send(None);
                        break;
                    }
                    if stderr_tx.send(Some(output)).is_err() {
                        // Send returns error only if data will never be received
                        break;
                    }
                });

                self.child = Some(ChildApp {
                    child,
                    stdout: Some(stdout_rx),
                    stderr: Some(stderr_rx),
                });

                Ok(String::new())
            }
            Err(err) => Err(err),
        }
    }

    fn update_output(&self, ui: &mut Ui) {
        match &self.output {
            Ok(output) => {
                let output = cansi::categorise_text(output);
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
            Err(output) => {
                ui.colored_label(Color32::RED, output);
            }
        }
    }

    fn read_output_from_child(&mut self) {
        if let Some(child) = &mut self.child {
            if let Some(receiver) = &mut child.stdout {
                for line in receiver.try_iter() {
                    if let Some(line) = line {
                        if let Ok(output) = &mut self.output {
                            output.push_str(&line);
                        }
                    } else {
                        child.stdout = None;
                        break;
                    }
                }
            }

            if let Some(receiver) = &mut child.stderr {
                for line in receiver.try_iter() {
                    if let Some(line) = line {
                        if let Ok(output) = &mut self.output {
                            output.push_str(&line);
                        }
                    } else {
                        child.stderr = None;
                        break;
                    }
                }
            }

            if child.stdout.is_none() && child.stderr.is_none() {
                self.kill_child()
            }
        }
    }

    fn kill_child(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.child.kill();
            self.child = None;
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
                ui.style_mut().spacing.text_edit_width = f32::MAX;
                self.state.update(ui);

                ui.horizontal(|ui| {
                    if ui
                        .add(Button::new("Run!").enabled(self.child.is_none()))
                        .clicked()
                    {
                        self.output = self.execute_command();
                    }

                    if self.child.is_some() {
                        if ui.button("Kill").clicked() {
                            self.kill_child();
                        }

                        let mut running_text = String::from("Running");
                        for _ in 0..((2.0 * ui.input().time) as i32 % 4) {
                            running_text.push('.')
                        }
                        ui.label(running_text);
                    }
                });

                self.read_output_from_child();
                self.update_output(ui);
            });
        });
    }

    fn on_exit(&mut self) {
        self.kill_child()
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

trait MyUi {
    fn error_style_if(&mut self, error: bool, f: impl FnOnce(&mut Ui));
    fn text_edit_singleline_hint(&mut self, text: &mut String, hint: impl ToString) -> Response;
}

impl MyUi for Ui {
    fn error_style_if(&mut self, error: bool, f: impl FnOnce(&mut Ui)) {
        let previous = if error {
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

        f(self);

        if let Some(previous) = previous {
            self.style_mut().visuals = previous;
        }
    }

    fn text_edit_singleline_hint(&mut self, text: &mut String, hint: impl ToString) -> Response {
        self.add(TextEdit::singleline(text).hint_text(hint))
    }
}
