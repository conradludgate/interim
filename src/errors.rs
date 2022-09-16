use scanlex::ScanError;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct DateError {
    details: String,
}

impl DateError {
    pub(crate) fn new(msg: impl Into<String>) -> Self {
        Self {
            details: msg.into(),
        }
    }
}

impl fmt::Display for DateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.details)
    }
}

impl Error for DateError {}

pub type DateResult<T> = Result<T, DateError>;

impl From<ScanError> for DateError {
    fn from(err: ScanError) -> DateError {
        DateError::new(err.to_string())
    }
}
impl From<&str> for DateError {
    fn from(err: &str) -> DateError {
        DateError::new(err)
    }
}
