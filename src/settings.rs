// Structs are marked as `#[non_exhaustive]` to allow
// to add other optionas alter withour breaking compatibility.

use eframe::egui::{self, style::Spacing, Style};
use std::borrow::Cow;

/// Settings for klask.
/// Is marked with `#[non_exhaustive]` so you must construct it like this
/// ```
/// # use klask::Settings;
/// let mut settings = Settings::default();
/// settings.enable_env = Some("Description".into());
/// ```
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Settings {
    /// Pass None to disable. Pass Some with a description to enable.
    /// Pass an empty String for no description.
    pub enable_env: Option<String>,
    /// Pass None to disable. Pass Some with a description to enable.
    /// Pass an empty String for no description.
    pub enable_stdin: Option<String>,
    /// Pass None to disable. Pass Some with a description to enable.
    /// Pass an empty String for no description.
    pub enable_working_dir: Option<String>,
    /// Pass a custom font to be used in the GUI.
    /// ```ignore
    /// let mut settings = Settings::default();
    /// settings.custom_font = Some(Cow::Borrowed(include_bytes!(r"FONT_PATH")));
    /// ```
    pub custom_font: Option<Cow<'static, [u8]>>,

    /// Override builtin strings. By default everything is in english.
    pub localization: Localization,

    /// Egui style used in GUI.
    pub style: Style,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enable_env: Option::default(),
            enable_stdin: Option::default(),
            enable_working_dir: Option::default(),
            custom_font: Option::default(),
            localization: Default::default(),
            style: Style {
                spacing: Spacing {
                    text_edit_width: f32::MAX,
                    item_spacing: egui::vec2(8.0, 8.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }
}

/// Localization for builtin strings.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Localization {
    /// Displays when the value is optional. Default is "(Optional)".
    pub optional: String,
    /// Button text for opening a dialog for file selection. Default is "Select file...".
    pub select_file: String,
    /// Button text for opening a dialog for directory selection. Default is "Select directory...".
    pub select_directory: String,
    /// Button text for creating a new field for multi-value arguments and environment variables. Default is "New value".
    pub new_value: String,
    /// Button text for resetting multi-value arguments. Default is "Reset".
    pub reset: String,
    /// Button text for resetting multi-value arguments to default. Default is "Reset to default".
    pub reset_to_default: String,
    /// Error text when an argument is requires. The argument name will be displayed between the two strings.
    /// Default is ("Argument '", "' is required").
    pub error_is_required: (String, String),
    /// Text for the arguments tab. Default is "Arguments".
    pub arguments: String,
    /// Text for the environment variables tab. Default is "Environment variables".
    pub env_variables: String,
    /// Error displayed when user tries to pass an environment variable with no name.
    /// Default is "Environment variable can't be empty".
    pub error_env_var_cant_be_empty: String,
    /// Text for the input tab. Default is "Input".
    pub input: String,
    /// Text for the button when user wants to write text for input in the input tab. Default is "Text".
    pub text: String,
    /// Text for the button when user wants to select file for input in the input tab. Default is "File".
    pub file: String,
    /// Text displayed as a hint for the working directory field. Default is "Working directory".
    pub working_directory: String,
    /// Button text for running the binary. Default is "Run".
    pub run: String,
    /// Button text for killing the binary. Default is "Kill".
    pub kill: String,
    /// Text that shows when the binary is running. There will be animated dots ("...") displayed after it.
    /// Default is "Running".
    pub running: String,
}

impl Default for Localization {
    fn default() -> Self {
        Self {
            optional: "(Optional)".into(),
            select_file: "Select file...".into(),
            select_directory: "Select directory...".into(),
            new_value: "New value".into(),
            reset: "Reset".into(),
            reset_to_default: "Reset to default".into(),
            error_is_required: ("Argument '".into(), "' is required".into()),
            arguments: "Arguments".into(),
            env_variables: "Environment variables".into(),
            error_env_var_cant_be_empty: "Environment variable can't be empty".into(),
            input: "Input".into(),
            text: "Text".into(),
            file: "File".into(),
            working_directory: "Working directory".into(),
            run: "Run".into(),
            kill: "Kill".into(),
            running: "Running".into(),
        }
    }
}
