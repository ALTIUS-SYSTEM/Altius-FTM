//! Integration tests against a live PostgreSQL.
//!
//! These run the real SQL through `PgStore` so a schema drift or a scoped
//! query that silently no-ops is caught by `cargo test`, not by a deploy.
//!
//! Skipped when `TEST_DATABASE_URL` (or `DATABASE_URL`) is unset:
//!
//! ```sh
//! docker compose up -d postgres
//! TEST_DATABASE_URL=postgres://altius:altius@localhost:5432/altius_test \
//!   cargo test -p altius-api
//! ```

use chrono::Utc;

use altius_core::{
    Coordinate, CostEntry, DailyReport, DeviceEvent, DeviceTime, EventReceipt, ExpenseCategory,
    GpsObservation, GpsQuality, GpsReview, GpsReviewClassification, GpsReviewReason, GpsSource,
    LhsStatus, Stop, StopAction, StopStatus, Task, TaskStatus, VehicleCheck, VehicleCondition,
};

use crate::config::{Config, GoogleRouteMode, KeycloakConfig, StoreBackend};
use crate::store::{PgStore, Store};

struct Fixture {
    pool: deadpool_postgres::Pool,
    store: Store,
}

impl Fixture {
    async fn new() -> Option<Self> {
        let url = std::env::var("TEST_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .ok()
            .filter(|u| !u.is_empty())?;
        let pool = crate::store::pg::connect(&url).await.ok()?;
        // Serialize migrations across parallel fixtures and fail loudly:
        // a migration error must kill the test, not skip it.
        static MIGRATE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
        {
            let _guard = MIGRATE_LOCK.lock().await;
            crate::store::pg::migrate(&pool)
                .await
                .expect("migrate test database");
        }
        Some(Self {
            store: Store::Postgres(PgStore::new(pool.clone())),
            pool,
        })
    }

    fn store(&self) -> &Store {
        &self.store
    }

    fn config(&self, org: &str, hub: &str, admin: &str) -> Config {
        Config {
            keycloak: KeycloakConfig {
                issuer: "http://localhost/realms/test".into(),
                jwks_url_override: None,
                token_url: "http://localhost/token".into(),
                audience: "test".into(),
            },
            store_backend: StoreBackend::Postgres,
            database_url: String::new(),
            typedb_database: String::new(),
            google_maps_api_key: None,
            google_route_mode: GoogleRouteMode::Directions,
            agent_state_secret: None,
            openrouter_api_key: None,
            openrouter_model: String::new(),
            cors_origins: vec![],
            allow_password_grant: false,
            default_org_id: org.into(),
            default_org_name: org.into(),
            default_hub_id: hub.into(),
            default_hub_name: hub.into(),
            default_admin_sub: admin.into(),
            keycloak_admin: None,
            notify: None,
            fcm_project_id: None,
            fcm_credentials_path: None,
            mceasy: None,
        }
    }

    /// Create an org and hub outside the default bootstrap path.
    async fn seed_org(&self, org: &str, hub: &str) {
        let client = self.pool.get().await.expect("seed connection");
        client
            .execute(
                "INSERT INTO organizations (id, name) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                &[&org, &org],
            )
            .await
            .expect("insert org");
        client
            .execute(
                "INSERT INTO hubs (id, name) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                &[&hub, &hub],
            )
            .await
            .expect("insert hub");
        client
            .execute(
                "INSERT INTO org_hubs (org_id, hub_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                &[&org, &hub],
            )
            .await
            .expect("link org/hub");
    }

    async fn seed_driver(&self, org: &str, hub: &str, sub: &str) {
        self.store()
            .provision_user(org, hub, sub, "Test Driver", "driver")
            .await
            .expect("provision driver");
    }
}

fn uniq(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::new_v4().simple())
}

fn task(id: &str, org: &str, hub: &str) -> Task {
    Task {
        id: id.into(),
        tenant_id: org.into(),
        hub_id: hub.into(),
        title: "Test task".into(),
        status: TaskStatus::Assigned,
        assignee_id: None,
        stops: vec![
            Stop {
                id: format!("{id}-s1"),
                sequence: 1,
                name: "Stop 1".into(),
                address: "Jl. Test 1".into(),
                location: Coordinate {
                    lat: -6.2,
                    lng: 106.8,
                },
                status: StopStatus::Pending,
                service_seconds: 60,
            },
            Stop {
                id: format!("{id}-s2"),
                sequence: 2,
                name: "Stop 2".into(),
                address: "Jl. Test 2".into(),
                location: Coordinate {
                    lat: -6.21,
                    lng: 106.81,
                },
                status: StopStatus::Pending,
                service_seconds: 90,
            },
        ],
        created_at: Utc::now(),
    }
}

fn event(
    id: &str,
    org: &str,
    hub: &str,
    task: &str,
    stop: Option<&str>,
    action: StopAction,
) -> DeviceEvent {
    DeviceEvent {
        event_id: id.into(),
        idempotency_key: format!("key-{id}"),
        tenant_id: org.into(),
        hub_id: hub.into(),
        driver_id: "driver-1".into(),
        device_id: "dev-1".into(),
        task_id: task.into(),
        stop_id: stop.map(str::to_string),
        action,
        time: DeviceTime {
            utc: Utc::now(),
            offset_minutes: 420,
        },
        location: None,
        accuracy_meters: None,
        payload: serde_json::json!({}),
    }
}

#[tokio::test]
async fn bootstrap_is_idempotent() {
    let Some(fx) = Fixture::new().await else {
        return;
    };
    let org = uniq("org");
    let hub = uniq("hub");
    let admin = uniq("admin");
    let config = fx.config(&org, &hub, &admin);
    fx.seed_org(&org, &hub).await;

    fx.store()
        .ensure_default_org_hub(&config)
        .await
        .expect("bootstrap");
    fx.store()
        .ensure_default_org_hub(&config)
        .await
        .expect("bootstrap twice");
    fx.store()
        .link_admin_user(&config)
        .await
        .expect("link admin");

    assert_eq!(
        fx.store()
            .organization_of(&admin)
            .await
            .expect("org of admin"),
        Some(org.clone())
    );
    assert_eq!(
        fx.store()
            .organization_and_hub_of(&admin)
            .await
            .expect("org and hub"),
        Some((org, hub))
    );
}

#[tokio::test]
async fn create_task_and_record_event() {
    let Some(fx) = Fixture::new().await else {
        return;
    };
    let org = uniq("org");
    let hub = uniq("hub");
    let driver = uniq("driver");
    fx.seed_org(&org, &hub).await;
    fx.seed_driver(&org, &hub, &driver).await;

    let t = task(&uniq("task"), &org, &hub);
    fx.store().create_task(&org, &t).await.expect("create task");
    let fetched = fx
        .store()
        .task_by_id(&org, &t.id)
        .await
        .expect("fetch task")
        .expect("task exists");
    assert_eq!(fetched["task"]["id"], t.id);
    assert_eq!(fetched["stops"].as_array().unwrap().len(), 2);

    let ev = event(
        &uniq("ev"),
        &org,
        &hub,
        &t.id,
        Some(&t.stops[0].id),
        StopAction::Arrive,
    );
    let receipt = fx
        .store()
        .record_event(&org, &driver, &ev)
        .await
        .expect("record event");
    assert!(matches!(receipt, EventReceipt::Accepted { .. }));

    // Replay with the same idempotency key returns the same server id.
    let replay = fx
        .store()
        .record_event(&org, &driver, &ev)
        .await
        .expect("replay event");
    match (receipt, replay) {
        (
            EventReceipt::Accepted {
                server_event_id: a, ..
            },
            EventReceipt::Accepted {
                server_event_id: b, ..
            },
        ) => assert_eq!(a, b),
        _ => panic!("expected accepted receipts"),
    }

    // The stop advanced to `arrived`.
    let mut fetched = fx
        .store()
        .task_by_id(&org, &t.id)
        .await
        .expect("fetch task")
        .unwrap();
    assert_eq!(fetched["stops"][0]["stage"], "arrived");
    assert_eq!(fetched["task"]["stage"], "in_progress");

    // Null stop_id resolves to the active stop (mobile single-stop clients).
    let ev2 = event(
        &uniq("ev2"),
        &org,
        &hub,
        &t.id,
        None,
        StopAction::StartActivity,
    );
    fx.store()
        .record_event(&org, &driver, &ev2)
        .await
        .expect("record without stop_id");
    fetched = fx
        .store()
        .task_by_id(&org, &t.id)
        .await
        .expect("fetch task")
        .unwrap();
    assert_eq!(fetched["stops"][0]["stage"], "working");
}

#[tokio::test]
async fn tenant_isolation() {
    let Some(fx) = Fixture::new().await else {
        return;
    };
    let org_a = uniq("orgA");
    let hub_a = uniq("hubA");
    let org_b = uniq("orgB");
    let hub_b = uniq("hubB");
    fx.seed_org(&org_a, &hub_a).await;
    fx.seed_org(&org_b, &hub_b).await;

    let t = task(&uniq("task"), &org_a, &hub_a);
    fx.store()
        .create_task(&org_a, &t)
        .await
        .expect("create task");

    // Another org cannot see the task.
    let tasks_b = fx.store().tasks_for_org(&org_b).await.expect("tasks for B");
    assert!(tasks_b.is_empty());
    assert!(
        fx.store()
            .task_by_id(&org_b, &t.id)
            .await
            .expect("task by id")
            .is_none()
    );

    // A driver in org B cannot record an event on org A's task.
    let driver_b = uniq("driverB");
    fx.seed_driver(&org_b, &hub_b, &driver_b).await;
    let ev = event(
        &uniq("ev"),
        &org_a,
        &hub_a,
        &t.id,
        Some(&t.stops[0].id),
        StopAction::Arrive,
    );
    let receipt = fx
        .store()
        .record_event(&org_b, &driver_b, &ev)
        .await
        .expect("record event");
    assert!(matches!(receipt, EventReceipt::Conflict { .. }));
}

#[tokio::test]
async fn hub_in_use_blocks_delete() {
    let Some(fx) = Fixture::new().await else {
        return;
    };
    let org = uniq("org");
    let hub = uniq("hub");
    fx.seed_org(&org, &hub).await;

    assert!(!fx.store().hub_in_use(&org, &hub).await.expect("hub in use"));
    let t = task(&uniq("task"), &org, &hub);
    fx.store().create_task(&org, &t).await.expect("create task");
    assert!(fx.store().hub_in_use(&org, &hub).await.expect("hub in use"));
}

#[tokio::test]
async fn reports_costs_and_vehicle_checks() {
    let Some(fx) = Fixture::new().await else {
        return;
    };
    let org = uniq("org");
    let hub = uniq("hub");
    let driver = uniq("driver");
    fx.seed_org(&org, &hub).await;
    fx.seed_driver(&org, &hub, &driver).await;

    fx.store()
        .record_cost(
            &CostEntry {
                id: uniq("cost"),
                category: ExpenseCategory::Fuel,
                amount_minor: 50000,
                currency: "IDR".into(),
                note: "Solar".into(),
                day: "2026-09-08".into(),
            },
            &driver,
        )
        .await
        .expect("record cost");

    fx.store()
        .record_daily_report(
            &DailyReport {
                day: "2026-09-08".into(),
                driver_id: driver.clone(),
                hub_id: hub.clone(),
                driver_name: "Test Driver".into(),
                vehicle_number: "B 1234 CD".into(),
                odometer_start: Some(100),
                odometer_end: Some(180),
                notes: "ok".into(),
                visited_task_ids: vec![],
                completed_stop_ids: vec![],
                costs: vec![],
                status: LhsStatus::Submitted,
                revision: 0,
            },
            &driver,
        )
        .await
        .expect("record report");

    fx.store()
        .record_vehicle_check(
            &org,
            &hub,
            &VehicleCheck {
                id: uniq("check"),
                day: "2026-09-08".into(),
                driver_id: driver.clone(),
                tenant_id: org.clone(),
                hub_id: hub.clone(),
                driver_name: "Test Driver".into(),
                license_plate: "B 1234 CD".into(),
                vehicle_type: "CDD".into(),
                km_start: 100,
                km_end: 180,
                condition: VehicleCondition::Good,
                items: vec![],
                notes: "ok".into(),
                service_date: None,
                kir_date: None,
                stnk_date: None,
            },
        )
        .await
        .expect("record vehicle check");

    assert_eq!(
        fx.store()
            .costs_for_driver(&driver, None)
            .await
            .expect("costs")
            .len(),
        1
    );
    assert_eq!(
        fx.store()
            .reports_for_driver(&driver)
            .await
            .expect("reports")
            .len(),
        1
    );
    assert_eq!(
        fx.store()
            .vehicle_checks_for_org(&org, None)
            .await
            .expect("checks")
            .len(),
        1
    );
}

#[tokio::test]
async fn gps_observations_reviews_and_prune() {
    let Some(fx) = Fixture::new().await else {
        return;
    };
    let org = uniq("org");
    let hub = uniq("hub");
    let driver = uniq("driver");
    fx.seed_org(&org, &hub).await;
    fx.seed_driver(&org, &hub, &driver).await;

    let obs = GpsObservation {
        id: uniq("obs"),
        tenant_id: org.clone(),
        hub_id: hub.clone(),
        driver_id: driver.clone(),
        vehicle_id: None,
        time: DeviceTime {
            utc: Utc::now(),
            offset_minutes: 420,
        },
        source: GpsSource::AppGps,
        quality: GpsQuality::Accurate,
        location: Some(Coordinate {
            lat: -6.2,
            lng: 106.8,
        }),
        accuracy_meters: Some(12.0),
        speed_mps: Some(8.0),
        mock_location_reported: Some(false),
    };
    fx.store()
        .record_gps_observation(&obs)
        .await
        .expect("record observation");

    let review = GpsReview {
        id: uniq("rev"),
        tenant_id: org.clone(),
        hub_id: hub.clone(),
        driver_id: driver.clone(),
        vehicle_id: None,
        app_observation_id: obs.id.clone(),
        vehicle_observation_id: None,
        classification: GpsReviewClassification::Consistent,
        separation_meters: Some(15.0),
        time_delta_seconds: Some(30.0),
        reason: GpsReviewReason::WithinTolerance,
        reviewed_by: None,
        created_at: Utc::now(),
    };
    fx.store()
        .record_gps_review(&review)
        .await
        .expect("record review");

    assert!(
        fx.store()
            .mark_gps_review_reviewed(&org, &review.id, "super")
            .await
            .expect("mark reviewed")
    );
    let reviews = fx.store().gps_reviews_for_org(&org).await.expect("reviews");
    assert_eq!(reviews.len(), 1);
    assert_eq!(reviews[0]["review"]["reviewed_by"], "super");

    let observations = fx
        .store()
        .gps_observations_for_org(&org, None, None)
        .await
        .expect("observations");
    assert_eq!(observations.len(), 1);

    fx.store()
        .prune_gps_observations_older_than(Utc::now() + chrono::Duration::days(1))
        .await
        .expect("prune");
    let observations = fx
        .store()
        .gps_observations_for_org(&org, None, None)
        .await
        .expect("observations after prune");
    assert!(observations.is_empty());
}

#[tokio::test]
async fn push_tokens_and_mceasy_sync() {
    let Some(fx) = Fixture::new().await else {
        return;
    };
    let org = uniq("org");
    let hub = uniq("hub");
    let driver = uniq("driver");
    fx.seed_org(&org, &hub).await;
    fx.seed_driver(&org, &hub, &driver).await;

    // Device ids must be unique per test — fixtures share the database.
    let device = uniq("dev");
    fx.store()
        .register_push_token(&driver, &device, "fcm-token-1")
        .await
        .expect("register token");
    assert_eq!(
        fx.store()
            .push_tokens_for_user(&driver)
            .await
            .expect("tokens"),
        vec!["fcm-token-1".to_string()]
    );

    // A second user cannot steal the device id.
    let other = uniq("other");
    fx.seed_driver(&org, &hub, &other).await;
    assert!(
        fx.store()
            .register_push_token(&other, &device, "fcm-token-2")
            .await
            .is_err()
    );

    // McEasy sync updates only the caller's org.
    fx.store()
        .mceasy_sync_driver(&org, &driver, "mc-driver-1")
        .await
        .expect("mceasy driver");
    fx.store()
        .mceasy_sync_vehicle(&org, "B 1234 CD", "mc-vehicle-1")
        .await
        .expect("mceasy vehicle");

    let orgs = fx.store().organizations().await.expect("organizations");
    assert!(orgs.contains(&org));
}
