use interim::{parse_duration, DateError};

#[test]
fn includes_the_unexpected_input() {
    let input = "7:26 bananas";
    let error = parse_duration(input).unwrap_err();
    assert_eq!(
        error.with_input(input).to_string(),
        "unexpected input \"bananas\" at bytes 5..12; expected 'am', 'pm', or 'Z' after the time"
    );
    assert_eq!(
        error.to_string(),
        "expected 'am', 'pm', or 'Z' after the time at position 5..12"
    );
}

#[test]
fn explains_unexpected_end_of_input() {
    let input = "7:";
    let error = parse_duration(input).unwrap_err();
    assert_eq!(
        error.with_input(input).to_string(),
        "unexpected end of input; expected minutes after ':'"
    );
}

#[test]
fn escapes_control_characters() {
    let input = "\u{1b}[31m";
    let error = DateError::ExpectedToken("a date", 0..input.len());
    assert_eq!(
        error.with_input(input).to_string(),
        "unexpected input \"\\u{1b}[31m\" at bytes 0..5; expected a date"
    );
}

#[test]
fn invalid_spans_fall_back_without_panicking() {
    for span in [
        0..10,
        1..2,
        core::ops::Range { start: 2, end: 1 },
        usize::MAX..usize::MAX,
    ] {
        let error = DateError::ExpectedToken("a date", span);
        assert_eq!(error.with_input("é").to_string(), error.to_string());
    }
}

#[test]
fn formats_errors_without_a_source_location() {
    for error in [
        DateError::MissingDate,
        DateError::MissingTime,
        DateError::UnexpectedDate,
        DateError::UnexpectedAbsoluteDate,
        DateError::UnexpectedTime,
    ] {
        assert_eq!(error.with_input("anything").to_string(), error.to_string());
    }
}
