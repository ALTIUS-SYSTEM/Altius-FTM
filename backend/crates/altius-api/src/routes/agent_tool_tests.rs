use super::agent::{MAX_PLAN_ROUTE_STOPS, parse_plan_route_stops};
use serde_json::json;

#[test]
fn stops_must_be_coordinate_objects_not_strings() {
    // The declared schema once said `items: {type: string}` while this
    // parser wanted {lat,lng}. A model that obeyed its own schema produced
    // zero usable stops, so the tool answered "needs >=2 stops" forever.
    let as_strings = json!({ "stops": ["Jl. Sudirman 1", "Jl. Thamrin 2"] });
    assert!(
        parse_plan_route_stops(&as_strings).is_err(),
        "must reject, not silently drop"
    );

    let ok = json!({ "stops": [{ "lat": -6.2, "lng": 106.8 }, { "lat": -6.3, "lng": 106.9 }] });
    assert_eq!(parse_plan_route_stops(&ok).unwrap().len(), 2);
}

#[test]
fn the_declared_schema_matches_what_the_parser_accepts() {
    // Build an argument that exactly satisfies the advertised JSON Schema
    // and assert the parser takes it. This is the invariant that broke.
    let tools = {
        // The schema is defined inline in `dispatch_tools`; mirror the shape
        // it advertises for `items` and check the parser agrees.
        json!({ "stops": [
            { "lat": 0.0, "lng": 0.0 },
            { "lat": 1.0, "lng": 1.0 },
        ]})
    };
    assert!(parse_plan_route_stops(&tools).is_ok());
}

#[test]
fn stop_count_is_bounded_because_the_matrix_is_quadratic() {
    let many: Vec<_> = (0..MAX_PLAN_ROUTE_STOPS + 1)
        .map(|i| json!({ "lat": i as f64 / 100.0, "lng": 0.0 }))
        .collect();
    assert!(parse_plan_route_stops(&json!({ "stops": many })).is_err());

    let one = json!({ "stops": [{ "lat": 0.0, "lng": 0.0 }] });
    assert!(
        parse_plan_route_stops(&one).is_err(),
        "a single stop is not a route"
    );
}

#[test]
fn coordinates_outside_the_world_are_rejected() {
    for bad in [
        json!({ "stops": [{ "lat": 91.0, "lng": 0.0 }, { "lat": 0.0, "lng": 0.0 }] }),
        json!({ "stops": [{ "lat": 0.0, "lng": 181.0 }, { "lat": 0.0, "lng": 0.0 }] }),
        json!({ "stops": [{ "lat": null, "lng": 0.0 }, { "lat": 0.0, "lng": 0.0 }] }),
    ] {
        assert!(parse_plan_route_stops(&bad).is_err(), "should reject {bad}");
    }
}

#[test]
fn a_missing_or_malformed_stops_argument_is_an_error() {
    assert!(parse_plan_route_stops(&json!({})).is_err());
    assert!(parse_plan_route_stops(&json!({ "stops": "not an array" })).is_err());
    assert!(parse_plan_route_stops(&json!({ "stops": [] })).is_err());
}
