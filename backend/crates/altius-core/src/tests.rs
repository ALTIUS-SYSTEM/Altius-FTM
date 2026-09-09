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

fn task_with(day: Option<&str>, time: Option<&str>) -> Task {
    Task {
        id: "t1".into(),
        tenant_id: "org".into(),
        hub_id: "hub".into(),
        title: "Deliver".into(),
        status: TaskStatus::Assigned,
        assignee_id: None,
        stops: vec![],
        created_at: Utc::now(),
        day: day.map(str::to_string),
        start_time: time.map(str::to_string),
        flow: None,
        priority: TaskPriority::Normal,
        notes: None,
    }
}

#[test]
fn schedule_accepts_a_well_formed_day_and_time() {
    assert!(task_with(Some("2026-09-09"), Some("08:30")).validate_schedule().is_ok());
    // Both are optional: a task with no schedule is still a valid task.
    assert!(task_with(None, None).validate_schedule().is_ok());
}

#[test]
fn schedule_rejects_free_text_dates_and_times() {
    // These are stored as TEXT, so without validation any of them would reach
    // the mobile app and the CSV export intact.
    for bad in ["09/09/2026", "2026-9-9", "2026-09-09T00:00:00Z", "tomorrow", ""] {
        assert!(
            task_with(Some(bad), None).validate_schedule().is_err(),
            "day {bad:?} should be rejected"
        );
    }
    for bad in ["8:30", "08:30:00", "24:00", "08:60", "0830", ""] {
        assert!(
            task_with(None, Some(bad)).validate_schedule().is_err(),
            "start_time {bad:?} should be rejected"
        );
    }
}

#[test]
fn schedule_bounds_flow_and_notes() {
    let mut t = task_with(None, None);
    t.notes = Some("x".repeat(MAX_TASK_NOTES));
    assert!(t.validate_schedule().is_ok());
    t.notes = Some("x".repeat(MAX_TASK_NOTES + 1));
    assert!(t.validate_schedule().is_err());

    let mut t = task_with(None, None);
    t.flow = Some("x".repeat(MAX_TASK_FLOW));
    assert!(t.validate_schedule().is_ok());
    t.flow = Some("x".repeat(MAX_TASK_FLOW + 1));
    assert!(t.validate_schedule().is_err());
}

#[test]
fn scheduled_day_falls_back_to_the_creation_date() {
    let t = task_with(Some("2026-01-02"), None);
    assert_eq!(t.scheduled_day(), "2026-01-02");
    let t = task_with(None, None);
    assert_eq!(t.scheduled_day(), Utc::now().date_naive().to_string());
}

#[test]
fn task_deserializes_without_the_schedule_fields() {
    // Clients written before these existed still post the old shape; a task
    // that fails to deserialize would be a hard break, not a missing field.
    let json = r#"{"id":"t1","tenant_id":"o","hub_id":"h","title":"x",
        "status":"assigned","assignee_id":null,"stops":[],
        "created_at":"2026-09-09T00:00:00Z"}"#;
    let t: Task = serde_json::from_str(json).expect("old payload must still parse");
    assert!(t.day.is_none());
    assert_eq!(t.priority, TaskPriority::Normal);
}
