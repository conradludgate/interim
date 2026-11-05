#![cfg(any(
    feature = "chrono_0_4",
    feature = "time_0_3",
    feature = "jiff_0_1",
    feature = "jiff_0_2"
))]

use interim::{datetime::DateTime, parse_date_string, Dialect};

fn check<D: DateTime + Clone>(base: D, micros: impl Fn(&D) -> u32) {
    for (fraction, expected) in [
        ("1", 100_000),
        ("12", 120_000),
        ("123", 123_000),
        ("1234", 123_400),
        ("12345", 123_450),
        ("123456", 123_456),
        ("1234567", 123_456),
        ("12345678901234567890", 123_456),
        ("000001", 1),
        ("0000009", 0),
        ("9999999", 999_999),
    ] {
        let input = format!("2024-01-02 03:04:05.{fraction}Z");
        let parsed = parse_date_string(&input, base.clone(), Dialect::Us).unwrap();
        assert_eq!(micros(&parsed), expected, "{input}");
    }
}

#[cfg(feature = "chrono_0_4")]
#[test]
fn chrono_fractional_seconds() {
    check(
        chrono::DateTime::UNIX_EPOCH,
        chrono::DateTime::timestamp_subsec_micros,
    );
}

#[cfg(feature = "time_0_3")]
#[test]
fn time_fractional_seconds() {
    check(time::OffsetDateTime::UNIX_EPOCH, |time| time.microsecond());
}

#[cfg(feature = "jiff_0_1")]
#[test]
fn jiff_0_1_fractional_seconds() {
    check(
        jiff_0_1::Timestamp::from_second(0)
            .unwrap()
            .to_zoned(jiff_0_1::tz::TimeZone::UTC),
        |time| u32::try_from(time.time().subsec_nanosecond()).unwrap() / 1_000,
    );
}

#[cfg(feature = "jiff_0_2")]
#[test]
fn jiff_0_2_fractional_seconds() {
    check(
        jiff_0_2::Timestamp::from_second(0)
            .unwrap()
            .to_zoned(jiff_0_2::tz::TimeZone::UTC),
        |time| u32::try_from(time.time().subsec_nanosecond()).unwrap() / 1_000,
    );
}
