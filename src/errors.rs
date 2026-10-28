use core::fmt;
use logos::Span;

#[cfg(feature = "fancy-diagnostics")]
mod fancy;

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

impl DateError {
    /// Attach the original input when displaying this error.
    ///
    /// The returned wrapper borrows both values and performs no allocation.
    /// Spans refer to byte offsets in `input`. If a span is invalid for the
    /// supplied input, display falls back to the error's standalone message.
    /// With the `fancy-diagnostics` feature, alternate display (`{:#}`) adds
    /// a source line and underline. Ordinary display remains compact.
    ///
    /// ```
    /// let input = "7:26 bananas";
    /// let error = interim::parse_duration(input).unwrap_err();
    /// assert_eq!(
    ///     error.with_input(input).to_string(),
    ///     "unexpected input \"bananas\" at bytes 5..12; expected 'am', 'pm', or 'Z' after the time"
    /// );
    /// ```
    #[must_use]
    pub fn with_input<'a>(&'a self, input: &'a str) -> DateErrorWithInput<'a> {
        DateErrorWithInput { error: self, input }
    }
}

/// A borrowed error and its original input, for source-aware display.
///
/// Construct this using [`DateError::with_input`]. Formatting never prints
/// directly to a terminal or takes ownership of the input.
#[derive(Clone, Copy, Debug)]
pub struct DateErrorWithInput<'a> {
    error: &'a DateError,
    input: &'a str,
}

impl fmt::Display for DateErrorWithInput<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(feature = "fancy-diagnostics")]
        if f.alternate() {
            if let Some(result) = fancy::format(self, f) {
                return result;
            }
        }
        match self.error {
            DateError::ExpectedToken(expected, span) => {
                if let Some(found) = self.input.get(span.clone()) {
                    return write!(
                        f,
                        "unexpected input {found:?} at bytes {span:?}; expected {expected}"
                    );
                }
            }
            DateError::EndOfText(expected) => {
                return write!(f, "unexpected end of input; expected {expected}");
            }
            _ => {}
        }
        fmt::Display::fmt(self.error, f)
    }
}

impl fmt::Display for DateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DateError::ExpectedToken(message, span) => {
                write!(f, "expected {message} at position {span:?}")
            }
            DateError::EndOfText(message) => {
                write!(f, "expected {message} at the end of the input")
            }
            DateError::MissingDate => f.write_str("date could not be parsed from input"),
            DateError::MissingTime => f.write_str("time could not be parsed from input"),
            DateError::UnexpectedDate => f.write_str("expected relative date, found a named date"),
            DateError::UnexpectedAbsoluteDate => {
                f.write_str("expected relative date, found an exact date")
            }
            DateError::UnexpectedTime => f.write_str("expected duration, found time"),
        }
    }
}

impl core::error::Error for DateError {}

pub type DateResult<T> = Result<T, DateError>;
