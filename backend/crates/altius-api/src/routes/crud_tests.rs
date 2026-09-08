use super::{valid_coordinate, valid_name};

#[test]
fn names_are_trimmed_and_bounded() {
    assert_eq!(valid_name("  Jakarta Pusat  ").unwrap(), "Jakarta Pusat");
    assert!(valid_name("").is_err());
    assert!(valid_name("   ").is_err(), "whitespace is not a name");
    assert!(valid_name(&"x".repeat(201)).is_err());
    assert!(valid_name(&"x".repeat(200)).is_ok());
}

#[test]
fn coordinates_reject_out_of_range_and_non_finite() {
    assert!(valid_coordinate(-6.2, 106.8).is_ok());
    assert!(valid_coordinate(90.0, 180.0).is_ok());
    assert!(valid_coordinate(90.1, 0.0).is_err());
    assert!(valid_coordinate(0.0, -180.1).is_err());
    // NaN passes every comparison it is asked, so test it explicitly.
    assert!(valid_coordinate(f64::NAN, 0.0).is_err());
    assert!(valid_coordinate(0.0, f64::INFINITY).is_err());
}
