use inflector::Inflector;

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Internal io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Internal error: no name in validation")]
    NoValidationName,
    #[error("Internal match error: {0}")]
    MatchError(clap::Error),
    #[error("Internal error: no child stdout or stderr")]
    NoStdoutOrStderr,
    #[error("Validation error in {}: '{}'", .name, .message)]
    ValidationError { name: String, message: String },
    #[error("{0}")]
    GuiError(String),
}

impl From<clap::Error> for ExecutionError {
    fn from(err: clap::Error) -> Self {
        match clap::Error::kind(&err) {
            clap::ErrorKind::ValueValidation => {
                if let Some(name) = err.info[0]
                    .split_once('<')
                    .and_then(|(_, suffix)| suffix.split_once('>'))
                    .map(|(prefix, _)| prefix.to_sentence_case())
                {
                    Self::ValidationError {
                        name,
                        message: err.info[2].clone(),
                    }
                } else {
                    Self::NoValidationName
                }
            }
            _ => Self::MatchError(err),
        }
    }
}

impl From<String> for ExecutionError {
    fn from(str: String) -> Self {
        Self::GuiError(str)
    }
}

impl From<&str> for ExecutionError {
    fn from(str: &str) -> Self {
        Self::GuiError(str.to_string())
    }
}
