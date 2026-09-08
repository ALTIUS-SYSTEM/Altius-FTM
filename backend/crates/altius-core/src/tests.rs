use crate::*;
use chrono::{TimeZone, Utc};

fn sample_event_snake() -> serde_json::Value {
    serde_json::json!({
        "event_id": "evt-1",
        "idempotency_key": "idem-1",
        "tenant_id": "tenant-1",
        "hub_id": "hub-1",
        "driver_id": "driver-1",
        "device_id": "phone-1",
        "task_id": "task-1",
        "stop_id": "stop-1",
        "action": "arrive",
        "time": { "utc": "2026-09-07T01:00:00Z", "offset_minutes": 420 },
        "location": { "lat": -6.2, "lng": 106.8 },
        "accuracy_meters": 5.0,
        "schema_version": 1,
        "device_sequence": 1,
        "expected_task_revision": 0,
        "observation_id": "obs-1",
        "payload": {}
    })
}

fn sample_event_camel_contract() -> serde_json::Value {
    serde_json::json!({
        "eventId": "evt-1",
        "idempotencyKey": "idem-1",
        "tenantId": "tenant-1",
        "hubId": "hub-1",
        "actorId": "driver-1",
        "deviceId": "phone-1",
        "taskId": "task-1",
        "stopId": "stop-1",
        "action": "arrive",
        "time": { "occurredAtUtc": "2026-09-07T01:00:00Z", "utcOffsetMinutes": 420 },
        "location": { "latitude": -6.2, "longitude": 106.8 },
        "accuracyMeters": 5.0,
        "schemaVersion": 1,
        "deviceSequence": 1,
        "expectedTaskRevision": 0,
        "observationId": "obs-1",
        "payload": {}
    })
}

fn assert_sample_event(ev: DeviceEvent) {
    assert_eq!(ev.event_id, "evt-1");
    assert_eq!(ev.driver_id, "driver-1");
    assert_eq!(ev.time.offset_minutes, 420);
    assert_eq!(
        ev.time.utc,
        Utc.with_ymd_and_hms(2026, 9, 7, 1, 0, 0).unwrap()
    );
    let loc = ev.location.expect("location");
    assert!((loc.lat - -6.2).abs() < f64::EPSILON);
    assert!((loc.lng - 106.8).abs() < f64::EPSILON);
    assert_eq!(ev.accuracy_meters, Some(5.0));
    assert_eq!(ev.schema_version, Some(1));
    assert_eq!(ev.device_sequence, Some(1));
    assert_eq!(ev.expected_task_revision, Some(0));
    assert_eq!(ev.observation_id.as_deref(), Some("obs-1"));
}

#[test]
fn device_event_deserializes_snake_case() {
    let ev: DeviceEvent = serde_json::from_value(sample_event_snake()).unwrap();
    assert_sample_event(ev);
}

#[test]
fn device_event_deserializes_camel_case_contract_aliases() {
    let ev: DeviceEvent = serde_json::from_value(sample_event_camel_contract()).unwrap();
    assert_sample_event(ev);
}

#[test]
fn device_event_serialize_stays_snake_case() {
    let ev: DeviceEvent = serde_json::from_value(sample_event_camel_contract()).unwrap();
    let out = serde_json::to_value(&ev).unwrap();
    assert!(out.get("event_id").is_some());
    assert!(out.get("eventId").is_none());
    assert!(out.get("driver_id").is_some());
    assert!(out.get("actorId").is_none());
    let time = out.get("time").unwrap();
    assert!(time.get("utc").is_some());
    assert!(time.get("occurredAtUtc").is_none());
    assert!(time.get("offset_minutes").is_some());
}

#[test]
fn coordinate_and_device_time_accept_both_shapes() {
    let snake: Coordinate =
        serde_json::from_value(serde_json::json!({ "lat": 1.0, "lng": 2.0 })).unwrap();
    let contract: Coordinate =
        serde_json::from_value(serde_json::json!({ "latitude": 1.0, "longitude": 2.0 })).unwrap();
    assert_eq!(snake, contract);

    let snake_t: DeviceTime = serde_json::from_value(serde_json::json!({
        "utc": "2026-09-07T01:00:00Z",
        "offset_minutes": 0
    }))
    .unwrap();
    let camel_t: DeviceTime = serde_json::from_value(serde_json::json!({
        "occurredAtUtc": "2026-09-07T01:00:00Z",
        "offsetMinutes": 0
    }))
    .unwrap();
    assert_eq!(snake_t.utc, camel_t.utc);
    assert_eq!(snake_t.offset_minutes, camel_t.offset_minutes);
}

#[test]
fn legal_stop_sequence() {
    let mut s = StopStatus::Pending;
    for a in [
        StopAction::Arrive,
        StopAction::StartActivity,
        StopAction::CompleteActivity,
        StopAction::Depart,
    ] {
        s = check_transition(s, a).unwrap();
    }
    assert_eq!(s, StopStatus::Departed);
}

#[test]
fn rejects_skipping_ahead() {
    assert!(check_transition(StopStatus::Pending, StopAction::CompleteActivity).is_err());
    assert!(check_transition(StopStatus::Arrived, StopAction::Arrive).is_err());
    assert!(check_transition(StopStatus::Working, StopAction::Depart).is_err());
}

#[test]
fn skip_only_from_pending() {
    assert!(check_transition(StopStatus::Pending, StopAction::Skip).is_ok());
    assert!(check_transition(StopStatus::Arrived, StopAction::Skip).is_err());
}
