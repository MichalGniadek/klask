use clap::error::ContextValue;
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
                let name =
                    if let Some(ContextValue::String(s)) = err.context().next().map(|(_, n)| n) {
                        s.split_once('<')
                            .and_then(|(_, suffix)| suffix.split_once('>'))
                            .map(|(prefix, _)| prefix.to_sentence_case())
                    } else {
                        return Self::NoValidationName;
                    };
                let Some(name) = name else {return Self::NoValidationName;};
                //let Some(ContextValue::String(message)) = err.context().nth(1).map(|(_, n)| n) else {
                //return Self::NoValidationName
                //};
                Self::ValidationError {
                    name,
                    message: "test".to_string(),
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
