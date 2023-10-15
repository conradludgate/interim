// use core::error::Error;

use logos::Span;

#[derive(Debug, PartialEq, Eq, Clone)]
/// Error types for parsing and processing date/time inputs
pub enum DateError {
    ExpectedToken(&'static str, Span),
    EndOfText(&'static str),
    MissingDate,
    MissingTime,

    UnexpectedDate,
    UnexpectedAbsoluteDate,
    UnexpectedTime,
}

#[cfg(feature = "std")]
mod std {
    use super::DateError;
    use std::fmt;

    impl fmt::Display for DateError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                DateError::ExpectedToken(message, span) => {
                    write!(f, "expected {message} as position {span:?}")
                }
                DateError::EndOfText(message) => {
                    write!(f, "expected {message} at the end of the input")
                }
                DateError::MissingDate => f.write_str("date could not be parsed from input"),
                DateError::MissingTime => f.write_str("time could not be parsed from input"),
                DateError::UnexpectedDate => {
                    f.write_str("expected relative date, found a named date")
                }
                DateError::UnexpectedAbsoluteDate => {
                    f.write_str("expected relative date, found an exact date")
                }
                DateError::UnexpectedTime => f.write_str("expected duration, found time"),
            }
        }
    }

    impl std::error::Error for DateError {}
}

pub type DateResult<T> = Result<T, DateError>;
