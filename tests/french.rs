//! French (`Language::French`) parsing acceptance tests.
#![cfg(feature = "chrono_0_4")]

use chrono::{DateTime, FixedOffset, TimeZone};
use interim::{parse_date_string, parse_duration_config, Config, Dialect, Interval, Language};

/// Base instant: 2018-03-21T11:00:00+02:00, a Wednesday (same as the English
/// acceptance suite).
fn base() -> DateTime<FixedOffset> {
    FixedOffset::east_opt(7200)
        .unwrap()
        .with_ymd_and_hms(2018, 3, 21, 11, 0, 0)
        .unwrap()
}

fn cfg() -> Config {
    Config::new()
        .dialect(Dialect::Uk)
        .language(Language::French)
}

#[track_caller]
fn assert_fr(input: &str, expected: &str) {
    let date = match parse_date_string(input, base(), cfg()) {
        Ok(date) => date,
        Err(e) => panic!("unexpected error parsing {input:?}\n\t{e:?}"),
    };
    let actual = date.format("%+").to_string();
    assert_eq!(actual, expected, "parsing {input:?}");
}

#[test]
fn relative_keywords() {
    assert_fr("maintenant", "2018-03-21T11:00:00+02:00");
    assert_fr("aujourd'hui", "2018-03-21T11:00:00+02:00");
    assert_fr("hier", "2018-03-20T11:00:00+02:00");
    assert_fr("demain", "2018-03-22T11:00:00+02:00");
}

#[test]
fn weekdays() {
    // plain weekday == the coming one
    assert_fr("vendredi", "2018-03-23T00:00:00+02:00");
    // with a time part
    assert_fr("vendredi 20:00", "2018-03-23T20:00:00+02:00");
}

#[test]
fn weekday_direction_prefix_and_postfix() {
    // French naturally postfixes the modifier: "vendredi prochain"
    // base is Wed 2018-03-21, Dialect::Uk -> "next" means Friday of next week
    assert_fr("vendredi prochain", "2018-03-30T00:00:00+02:00");
    assert_fr("vendredi dernier", "2018-03-16T00:00:00+02:00");
    // prefix form also accepted, mirroring English
    assert_fr("prochain vendredi", "2018-03-30T00:00:00+02:00");
    assert_fr("dernier vendredi", "2018-03-16T00:00:00+02:00");
    // postfix modifier followed by a time part
    assert_fr("vendredi dernier 09:30", "2018-03-16T09:30:00+02:00");
}

#[test]
fn month_direction_postfix() {
    // "avril prochain" -> April next (April is later this year -> stays 2018)
    assert_fr("avril prochain", "2018-04-01T00:00:00+02:00");
    // "mars dernier": March 1 is already past on 2018-03-21, so "last" leaves
    // it in the current year (same rule as English "last <month>")
    assert_fr("mars dernier", "2018-03-01T00:00:00+02:00");
}

#[test]
fn month_names_with_accents() {
    assert_fr("15 juin 2020", "2020-06-15T00:00:00+02:00");
    assert_fr("1 avril 2018", "2018-04-01T00:00:00+02:00");
    // accented input
    assert_fr("15 février 2019", "2019-02-15T00:00:00+02:00");
    assert_fr("3 août 2021", "2021-08-03T00:00:00+02:00");
    assert_fr("25 décembre 2022", "2022-12-25T00:00:00+02:00");
    // accent-optional: same words without diacritics still match
    assert_fr("15 fevrier 2019", "2019-02-15T00:00:00+02:00");
    assert_fr("3 aout 2021", "2021-08-03T00:00:00+02:00");
    // juin / juillet must not collide (both start "jui")
    assert_fr("10 juillet 2020", "2020-07-10T00:00:00+02:00");
}

#[test]
fn month_name_alone() {
    // a bare month name -> the 1st (April is later this year)
    assert_fr("avril", "2018-04-01T00:00:00+02:00");
}

#[test]
fn future_durations() {
    assert_fr("3 jours", "2018-03-24T11:00:00+02:00");
    assert_fr("2 semaines", "2018-04-04T11:00:00+02:00");
    assert_fr("3 heures", "2018-03-21T14:00:00+02:00");
    assert_fr("6 mois", "2018-09-21T00:00:00+02:00");
}

#[test]
fn il_y_a_past() {
    assert_fr("il y a 3 jours", "2018-03-18T11:00:00+02:00");
    assert_fr("il y a 6 mois", "2017-09-21T00:00:00+02:00");
    assert_fr("il y a 3 heures", "2018-03-21T08:00:00+02:00");
}

#[test]
fn duration_only() {
    let c = cfg();
    assert_eq!(
        parse_duration_config("il y a 2 jours", c).unwrap(),
        Interval::Days(-2)
    );
    assert_eq!(
        parse_duration_config("3 heures", c).unwrap(),
        Interval::Seconds(3 * 60 * 60)
    );
    assert_eq!(
        parse_duration_config("2 semaines", c).unwrap(),
        Interval::Days(14)
    );
    assert_eq!(
        parse_duration_config("5 minutes", c).unwrap(),
        Interval::Seconds(5 * 60)
    );
    assert_eq!(
        parse_duration_config("2 ans", c).unwrap(),
        Interval::Months(24)
    );
}

#[test]
fn iso_dates_still_work() {
    // language-neutral formats are unaffected by the French vocabulary
    assert_fr("2017-06-30", "2017-06-30T00:00:00+02:00");
    assert_fr("30/06/17", "2017-06-30T00:00:00+02:00");
}
