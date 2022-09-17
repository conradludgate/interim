// use core::error::Error;

use logos::Span;

#[derive(Debug, PartialEq, Eq)]
pub enum DateError {
    UnexpectedToken(&'static str, Span),
    UnexpectedEndOfText(&'static str),
    MissingDate,
    MissingTime,

    UnexpectedDate,
    UnexpectedAbsoluteDate,
    UnexpectedTime,
}

// impl DateError {
//     pub(crate) fn new(msg: impl Into<String>) -> Self {
//         Self {
//             details: msg.into(),
//         }
//     }
// }

// impl fmt::Display for DateError {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         f.write_str(&self.details)
//     }
// }

// impl Error for DateError {}

pub type DateResult<T> = Result<T, DateError>;

// impl From<&str> for DateError {
//     fn from(err: &str) -> DateError {
//         DateError::new(err)
//     }
// }
