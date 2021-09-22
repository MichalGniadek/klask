use inflector::Inflector;

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
