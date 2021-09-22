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
use inflector::Inflector;
use linkify::{LinkFinder, LinkKind};
use std::{
    io::{BufRead, BufReader, Read},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
};

pub struct ChildApp {
    child: Child,
    stdout: Option<Receiver<Option<String>>>,
    stderr: Option<Receiver<Option<String>>>,
    output: String,
}

impl ChildApp {
    pub fn run(cmd: &mut Command) -> Result<Self, ExecuteError> {
        let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;

        let stdout =
            Self::spawn_thread_reader(child.stdout.take().ok_or(ExecuteError::NoStdoutOrStderr)?);
        let stderr =
            Self::spawn_thread_reader(child.stderr.take().ok_or(ExecuteError::NoStdoutOrStderr)?);

        Ok(ChildApp {
            child,
            stdout: Some(stdout),
            stderr: Some(stderr),
            output: String::new(),
        })
    }

    pub fn read(&mut self) -> &str {
        let mut read = |stdio: &mut Option<Receiver<Option<String>>>| {
            if let Some(receiver) = stdio {
                for line in receiver.try_iter() {
                    if let Some(line) = line {
                        self.output.push_str(&line);
                    } else {
                        *stdio = None;
                        return;
                    }
                }
            }
        };
        read(&mut self.stdout);
        read(&mut self.stderr);
        &self.output
    }

    pub fn kill(&mut self) {
        let _ = self.child.kill();
        self.stdout = None;
        self.stderr = None;
    }

    fn spawn_thread_reader<R: Read + Send + Sync + 'static>(stdio: R) -> Receiver<Option<String>> {
        let mut reader = BufReader::new(stdio);
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || loop {
            let mut output = String::new();
            if let Ok(0) = reader.read_line(&mut output) {
                // End of output
                let _ = tx.send(None);
                break;
            }
            // Send returns error only if data will never be received
            if tx.send(Some(output)).is_err() {
                break;
            }
        });
        rx
    }
}

#[derive(Debug, Clone)]
pub struct ValidationErrorInfo {
    name: String,
    message: String,
}

pub trait ValidationErrorInfoTrait {
    fn is<'a>(&'a self, name: &str) -> Option<&'a String>;
}

impl ValidationErrorInfoTrait for Option<ValidationErrorInfo> {
    fn is<'a>(&'a self, name: &str) -> Option<&'a String> {
        self.as_ref()
            .map(
                |ValidationErrorInfo { name: n, message }| {
                    if n == name {
                        Some(message)
                    } else {
                        None
                    }
                },
            )
            .flatten()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExecuteError {
    #[error("Internal io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Internal error: no name in validation")]
    NoValidationName,
    #[error("Internal match error: {0}")]
    MatchError(clap::Error),
    #[error("Internal error: no child stdout or stderr")]
    NoStdoutOrStderr,
    #[error("Validation error in {}: '{}'", .0.name, .0.message)]
    ValidationError(ValidationErrorInfo),
    #[error("{0}")]
    GuiError(String),
}

impl From<clap::Error> for ExecuteError {
    fn from(err: clap::Error) -> Self {
        match err.kind {
            clap::ErrorKind::ValueValidation => {
                if let Some(name) = err.info[0]
                    .split_once('<')
                    .and_then(|(_, suffix)| suffix.split_once('>'))
                    .map(|(prefix, _)| prefix.to_sentence_case())
                {
                    ExecuteError::ValidationError(ValidationErrorInfo {
                        name,
                        message: err.info[2].clone(),
                    })
                } else {
                    ExecuteError::NoValidationName
                }
            }
            _ => ExecuteError::MatchError(err),
        }
    }
}

impl From<String> for ExecuteError {
    fn from(str: String) -> Self {
        Self::GuiError(str)
    }
}

pub struct Klask {
    output: Option<Result<ChildApp, ExecuteError>>,
    state: AppState,
    validation_error: Option<ValidationErrorInfo>,
    // This isn't a generic lifetime because eframe::run_native() requires
    // a 'static lifetime because boxed trait objects default to 'static
    app: App<'static>,
}

// Public interface
impl Klask {
    pub fn run_app(app: App<'static>, f: impl FnOnce(&ArgMatches)) {
        // Wrap app in another in case no arguments is a valid configuration
        match App::new("outer").subcommand(app.clone()).try_get_matches() {
            Ok(matches) => match matches.subcommand_matches(app.get_name()) {
                // Called with arguments -> start user program
                Some(m) => f(m),
                // Called with no arguments -> start gui
                None => {
                    let klask = Self {
                        output: None,
                        state: AppState::new(&app),
                        validation_error: None,
                        app,
                    };
                    let native_options = eframe::NativeOptions::default();
                    eframe::run_native(Box::new(klask), native_options);
                }
            },
            Err(err) => panic!(
                "Internal error, arguments should've been empty or verified by the GUI app {:#?}",
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
}

impl Klask {
    fn execute(&mut self) -> Result<ChildApp, ExecuteError> {
        // Call the same executable, with subcommand equal to inner app's name
        let mut cmd = Command::new(std::env::current_exe()?);
        cmd.arg(self.app.get_name());
        let mut cmd = self.state.set_cmd_args(cmd)?;

        // Check for validation errors
        self.app.clone().try_get_matches_from(cmd.get_args())?;

        ChildApp::run(&mut cmd)
    }

    fn update_output(&mut self, ui: &mut Ui) {
        match &mut self.output {
            Some(Ok(c)) => ui.ansi_label(c.read()),
            Some(Err(err)) => {
                ui.colored_label(Color32::RED, err.to_string());
            }
            _ => {}
        }
    }

    fn kill_child(&mut self) {
        if let Some(Ok(child)) = &mut self.output {
            child.kill();
        }
    }

    fn is_child_running(&self) -> bool {
        match &self.output {
            Some(Ok(c)) => c.stdout.is_some() || c.stderr.is_some(),
            _ => false,
        }
    }
}

impl epi::App for Klask {
    fn name(&self) -> &str {
        self.app.get_name()
    }

    fn update(&mut self, ctx: &eframe::egui::CtxRef, _frame: &mut epi::Frame<'_>) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.style_mut().spacing.text_edit_width = f32::MAX;
            egui::ScrollArea::auto_sized().show(ui, |ui| {
                self.state.update(ui, &mut self.validation_error);

                ui.horizontal(|ui| {
                    if ui
                        .add(Button::new("Run!").enabled(!self.is_child_running()))
                        .clicked()
                    {
                        self.output = Some(self.execute());
                        self.validation_error =
                            if let Some(Err(ExecuteError::ValidationError(info))) = &self.output {
                                Some(info.clone())
                            } else {
                                None
                            };
                    }

                    if self.is_child_running() {
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

                self.update_output(ui);
            });
        });
    }

    fn on_exit(&mut self) {
        self.kill_child()
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

trait MyUi {
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
        F: FnMut(&mut Ui, &mut T) -> Response;
}

impl MyUi for Ui {
    fn error_style_if<F: FnOnce(&mut Ui) -> R, R>(&mut self, is_error: bool, f: F) -> R {
        if is_error {
            let visuals = &mut self.style_mut().visuals;
            visuals.widgets.inactive.bg_stroke.color = Color32::RED;
            visuals.widgets.inactive.bg_stroke.width = 1.0;
            visuals.widgets.hovered.bg_stroke.color = Color32::RED;
            visuals.widgets.active.bg_stroke.color = Color32::RED;
            visuals.widgets.open.bg_stroke.color = Color32::RED;
            visuals.widgets.noninteractive.bg_stroke.color = Color32::RED;
            visuals.selection.stroke.color = Color32::RED;
        }

        let ret = f(self);

        if is_error {
            self.reset_style();
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
        F: FnMut(&mut Ui, &mut T) -> Response,
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
                        ui.add_space(20.0);
                        if ui.button("Reset to default").clicked() {
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
