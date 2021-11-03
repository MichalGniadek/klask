/// Settings for klask.
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
    /// ```no_run
    /// let settings = Settings {
    ///     custom_font: Some(include_bytes!(r"FONT_PATH")),
    ///     ..Default::default()
    /// };
    ///```
    pub custom_font: Option<&'static [u8]>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enable_env: None,
            enable_stdin: None,
            enable_working_dir: None,
            custom_font: None,
        }
    }
}
