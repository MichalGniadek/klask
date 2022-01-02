/// Settings for klask.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
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
    /// let settings = Settings {
    ///     custom_font: Some(include_bytes!(r"FONT_PATH")),
    ///     ..Default::default()
    /// };
    ///```
    pub custom_font: Option<&'static [u8]>,
}
