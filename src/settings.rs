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
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enable_env: None,
            enable_stdin: None,
            enable_working_dir: None,
        }
    }
}
