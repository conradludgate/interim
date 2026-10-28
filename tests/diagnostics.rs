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
        assert_eq!(format!("{:#}", error.with_input("é")), error.to_string());
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
        assert_eq!(
            format!("{:#}", error.with_input("anything")),
            error.to_string()
        );
    }
}

#[cfg(not(feature = "fancy-diagnostics"))]
#[test]
fn alternate_display_is_compact_without_the_feature() {
    let error = parse_duration("bananas").unwrap_err();
    let report = error.with_input("bananas");
    assert_eq!(format!("{report:#}"), report.to_string());
}

#[cfg(feature = "fancy-diagnostics")]
mod fancy {
    use super::*;

    #[test]
    fn highlights_the_unexpected_token() {
        let input = "7:26 bananas";
        let error = parse_duration(input).unwrap_err();
        assert_eq!(
            format!("{:#}", error.with_input(input)),
            concat!(
                "unexpected input \"bananas\"\n",
                "  |\n",
                "1 | 7:26 bananas\n",
                "  |      ^^^^^^^ expected 'am', 'pm', or 'Z' after the time"
            )
        );
    }

    #[test]
    fn expands_tabs_before_and_inside_the_span() {
        let input = "a\tword\tend";
        let error = DateError::ExpectedToken("a date", 2..7);
        assert_eq!(
            format!("{:#}", error.with_input(input)),
            concat!(
                "unexpected input \"word\\t\"\n",
                "  |\n",
                "1 | a   word    end\n",
                "  |     ^^^^^^^^ expected a date"
            )
        );
    }

    #[test]
    fn selects_the_source_line_and_aligns_the_gutter() {
        let input = format!("{}bananas", "\n".repeat(9));
        let error = parse_duration(&input).unwrap_err();
        assert_eq!(
            format!("{:#}", error.with_input(&input)),
            concat!(
                "unexpected input \"bananas\"\n",
                "   |\n",
                "10 | bananas\n",
                "   | ^^^^^^^ expected a weekday, month name, or relative time unit"
            )
        );
    }

    #[test]
    fn supports_crlf_source_lines() {
        // Test rendering independently of the parser's whitespace grammar.
        let input = "first\r\n  bananas\r\n";
        let error = DateError::ExpectedToken("a date", 9..16);
        assert_eq!(
            format!("{:#}", error.with_input(input)),
            concat!(
                "unexpected input \"bananas\"\n",
                "  |\n",
                "2 |   bananas\n",
                "  |   ^^^^^^^ expected a date"
            )
        );
    }

    #[test]
    fn points_at_missing_input_including_an_empty_last_line() {
        for (input, expected) in [
            (
                "7:",
                "unexpected end of input\n  |\n1 | 7:\n  |   ^ expected minutes after ':'",
            ),
            (
                "7:\n",
                "unexpected end of input\n  |\n2 | \n  | ^ expected minutes after ':'",
            ),
            (
                "",
                "unexpected end of input\n  |\n1 | \n  | ^ expected a date or duration",
            ),
        ] {
            let error = parse_duration(input).unwrap_err();
            assert_eq!(format!("{:#}", error.with_input(input)), expected);
        }
    }

    #[test]
    fn marks_a_zero_width_span_with_one_caret() {
        let error = DateError::ExpectedToken("a date", 1..1);
        assert_eq!(
            format!("{:#}", error.with_input("ab")),
            "unexpected input \"\"\n  |\n1 | ab\n  |  ^ expected a date"
        );
    }

    #[test]
    fn falls_back_for_unicode_controls_and_spans_crossing_lines() {
        for (input, span) in [("é", 0..2), ("a\u{1b}b", 0..1), ("a\nb", 0..3)] {
            let error = DateError::ExpectedToken("a date", span);
            let report = error.with_input(input);
            assert_eq!(format!("{report:#}"), report.to_string());
            assert!(!format!("{report:#}").contains('\u{1b}'));
        }
    }
}
