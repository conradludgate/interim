use interim::parse_duration;

#[test]
fn date_errors_implement_core_and_std_error() {
    let error = parse_duration("7:26 bananas").unwrap_err();
    let core_error: &dyn core::error::Error = &error;
    assert!(core_error.source().is_none());
    assert_eq!(core_error.to_string(), error.to_string());

    // The std trait is the same trait, so existing std consumers still work.
    let std_error: &dyn std::error::Error = &error;
    assert_eq!(std_error.to_string(), error.to_string());
}
