use interim::{parse_duration, DateError, Interval};

#[test]
fn unknown_identifier_error() {
    for (input, span) in [("bananas", 0..7), ("  bananas", 2..9)] {
        let error = parse_duration(input).unwrap_err();
        assert_eq!(
            error,
            DateError::ExpectedToken("supported identifier", span.clone())
        );
        assert_eq!(
            error.to_string(),
            format!("expected supported identifier at position {span:?}")
        );
    }
}

#[test]
fn malformed_time_error_messages() {
    for (input, expected) in [
        ("7:26 bananas", "expected Z/am/pm at position 5..12"),
        ("7:26:30 1", "expected timezone at position 8..9"),
        ("7.26 bananas", "expected am/pm at position 5..12"),
    ] {
        assert_eq!(
            parse_duration(input).unwrap_err().to_string(),
            expected,
            "{input}"
        );
    }
}

#[test]
fn acceptance() {
    macro_rules! assert_duration {
        ($s:literal, $expect:expr) => {
            let dur = parse_duration($s).unwrap();
            assert_eq!(dur, $expect);
        };
    }
    macro_rules! assert_duration_err {
        ($s:literal, $expect:expr) => {
            let err = parse_duration($s).unwrap_err();
            assert_eq!(err, $expect);
        };
    }

    assert_duration!("1 seconds", Interval::Seconds(1));
    assert_duration!("24 seconds", Interval::Seconds(24));
    assert_duration!("34 s", Interval::Seconds(34));
    assert_duration!("34 sec", Interval::Seconds(34));

    assert_duration!("6h", Interval::Seconds(6 * 3600));
    assert_duration!("4 hours ago", Interval::Seconds(-4 * 3600));
    assert_duration!("5 min", Interval::Seconds(5 * 60));
    assert_duration!("10m", Interval::Seconds(10 * 60));
    assert_duration!("15m ago", Interval::Seconds(-15 * 60));

    assert_duration!("1 day", Interval::Days(1));
    assert_duration!("2 days ago", Interval::Days(-2));
    assert_duration!("3 weeks", Interval::Days(21));
    assert_duration!("2 weeks ago", Interval::Days(-14));

    assert_duration!("1 month", Interval::Months(1));
    assert_duration!("6 months", Interval::Months(6));
    assert_duration!("8 years", Interval::Months(12 * 8));

    // errors
    assert_duration_err!("2020-01-01", DateError::UnexpectedAbsoluteDate);
    assert_duration_err!("2 days 15:00", DateError::UnexpectedTime);
    assert_duration_err!("tuesday", DateError::UnexpectedDate);
    assert_duration_err!(
        "bananas",
        DateError::ExpectedToken("supported identifier", 0..7)
    );
}
