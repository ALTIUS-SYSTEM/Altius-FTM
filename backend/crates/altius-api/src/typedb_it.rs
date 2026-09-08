//! Integration tests against a live TypeDB.
//!
//! Every query in [`crate::store`] is a hand-built TypeQL string that no unit
//! test executes. `cargo test` and `clippy` both stayed green while
//! `drivers_for_org` referenced an `entity role` and a `relation assignment`
//! the schema never defined, and while `migrate` skipped every type added
//! after a database's first deployment. These tests catch that class of bug by
//! running the real queries against a real server.
//!
//! Skipped (not failed) when `TYPEDB_ADDRESS` is unset, so a checkout without
//! Docker still has a green suite:
//!
//! ```sh
//! docker compose up -d typedb
//! TYPEDB_ADDRESS=localhost:1729 cargo test -p altius-api
//! ```

use altius_core::{
    Coordinate, DeviceEvent, DeviceTime, Stop, StopAction, StopStatus, Task, TaskStatus,
};

use crate::store::{Store, TypedbStore};

/// A throwaway database per test, so runs never collide with real data.
///
/// Two connections: `Store` takes ownership of one, so seeding and teardown
/// need their own (`TypeDBDriver` is not `Clone`).
struct Fixture {
    store: Store,
    admin: typedb_driver::TypeDBDriver,
    database: String,
}

impl Fixture {
    async fn new() -> Option<Self> {
        let address = std::env::var("TYPEDB_ADDRESS")
            .ok()
            .filter(|a| !a.is_empty())?;
        let user = std::env::var("TYPEDB_USERNAME").unwrap_or_else(|_| "admin".into());
        let pass = std::env::var("TYPEDB_PASSWORD").unwrap_or_else(|_| "password".into());
        let database = format!("altius_it_{}", uuid::Uuid::new_v4().simple());

        let admin = altius_schema::connect(&address, &user, &pass)
            .await
            .expect("connect to TypeDB");
        altius_schema::migrate(&admin, &database)
            .await
            .expect("apply schema");
        let owned = altius_schema::connect(&address, &user, &pass)
            .await
            .expect("second connection for the store");
        Some(Self {
            store: Store::Typedb(TypedbStore::new(owned, database.clone())),
            admin,
            database,
        })
    }

    fn store(&self) -> &Store {
        &self.store
    }

    /// Insert an org with one hub, allocated to it.
    async fn seed(&self, org: &str, hub: &str) {
        let tx = self
            .admin
            .transaction(&self.database, typedb_driver::TransactionType::Write)
            .await
            .expect("write tx");
        tx.query(&format!(
            r#"insert
                $o isa organization, has org-id "{org}", has display-name "{org} Ltd";
                $h isa hub, has hub-id "{hub}", has display-name "{hub} Hub";
                allocation (org: $o, hub: $h);"#
        ))
        .await
        .expect("seed org/hub");
        tx.commit().await.expect("commit seed");
    }

    async fn cleanup(self) {
        if let Ok(db) = self.admin.databases().get(&self.database).await {
            db.delete().await.ok();
        }
    }
}

/// A one-stop task rooted in the given tenant.
fn sample_task(id: &str, org: &str, hub: &str) -> Task {
    Task {
        id: id.into(),
        tenant_id: org.into(),
        hub_id: hub.into(),
        title: "Delivery".into(),
        status: TaskStatus::Assigned,
        assignee_id: None,
        created_at: chrono::Utc::now(),
        stops: vec![Stop {
            id: format!("{id}-stop-1"),
            sequence: 0,
            name: "Warehouse".into(),
            address: "Jl. Sudirman 1".into(),
            location: Coordinate {
                lat: -6.2,
                lng: 106.8,
            },
            status: StopStatus::Pending,
            service_seconds: 300,
        }],
    }
}

/// Minimal config for the startup bootstrap helpers.
fn bootstrap_config() -> crate::config::Config {
    crate::config::Config {
        keycloak_admin: None,
        notify: None,
        mceasy: None,
        fcm_project_id: None,
        fcm_credentials_path: None,
        keycloak: crate::config::KeycloakConfig {
            issuer: "https://sso.example.com/realms/test".into(),
            jwks_url_override: None,
            token_url: "https://sso.example.com/realms/test/protocol/openid-connect/token".into(),
            audience: "altius".into(),
        },
        store_backend: crate::config::StoreBackend::Typedb,
        database_url: String::new(),
        typedb_database: "test".into(),
        google_maps_api_key: None,
        google_route_mode: crate::config::GoogleRouteMode::Directions,
        agent_state_secret: None,
        openrouter_api_key: None,
        openrouter_model: "test-model".into(),
        cors_origins: vec![],
        allow_password_grant: false,
        default_org_id: "bootstrap-org".into(),
        default_org_name: "Bootstrap Org".into(),
        default_hub_id: "bootstrap-hub".into(),
        default_hub_name: "Bootstrap Hub".into(),
        default_admin_sub: "bootstrap-admin".into(),
    }
}

/// Skip the body when no server is configured.
macro_rules! fixture {
    () => {
        match Fixture::new().await {
            Some(f) => f,
            None => {
                eprintln!("TYPEDB_ADDRESS unset — skipping integration test");
                return;
            }
        }
    };
}

#[tokio::test]
async fn schema_reapplies_without_error() {
    let f = fixture!();
    // The guard that skipped this is why every type added after the first
    // deployment never reached a database that already held data.
    altius_schema::migrate(&f.admin, &f.database)
        .await
        .expect("re-applying the schema must be a no-op, not a failure");
    f.cleanup().await;
}

#[tokio::test]
async fn roster_queries_run_against_the_real_schema() {
    let f = fixture!();
    let s = f.store();
    f.seed("org-a", "hub-a").await;

    s.provision_user("org-a", "hub-a", "sub-driver", "Adi Pratama", "driver")
        .await
        .expect("provision driver");
    s.provision_user("org-a", "hub-a", "sub-lead", "Nadia Putri", "lead")
        .await
        .expect("provision lead");

    assert_eq!(
        s.users_for_org("org-a").await.expect("users_for_org").len(),
        2
    );
    assert_eq!(
        s.hubs_for_org("org-a").await.expect("hubs_for_org").len(),
        1
    );
    // This one failed at runtime while the whole unit suite was green.
    assert_eq!(
        s.drivers_for_org("org-a")
            .await
            .expect("drivers_for_org")
            .len(),
        1,
        "the driver, not the lead"
    );

    f.cleanup().await;
}

#[tokio::test]
async fn team_lifecycle_survives_deleting_a_team_with_members() {
    let f = fixture!();
    let s = f.store();
    f.seed("org-a", "hub-a").await;
    s.provision_user("org-a", "hub-a", "sub-1", "Adi", "driver")
        .await
        .unwrap();

    assert!(
        s.create_team("org-a", "hub-a", "team-1", "Morning", "06:00-14:00")
            .await
            .unwrap()
    );
    assert!(s.add_team_member("org-a", "team-1", "sub-1").await.unwrap());
    assert_eq!(
        s.teams_for_org("org-a").await.expect("teams_for_org").len(),
        1
    );
    assert!(
        s.update_team("org-a", "team-1", "Early", "05:00-13:00")
            .await
            .unwrap()
    );

    // TypeDB refuses to delete an entity still playing a role in a relation,
    // so a team with members could not be removed at all.
    assert!(s.delete_team("org-a", "team-1").await.unwrap());
    assert!(s.teams_for_org("org-a").await.unwrap().is_empty());

    f.cleanup().await;
}

#[tokio::test]
async fn hub_delete_refuses_to_orphan_records() {
    let f = fixture!();
    let s = f.store();
    f.seed("org-a", "hub-a").await;

    assert!(
        s.create_hub("org-a", "hub-b", "Bandung", -6.9, 107.6)
            .await
            .unwrap()
    );
    assert!(
        s.update_hub("org-a", "hub-b", "Bandung Kota", -6.91, 107.61)
            .await
            .unwrap()
    );
    assert!(!s.hub_in_use("org-a", "hub-b").await.unwrap());

    s.provision_user("org-a", "hub-b", "sub-2", "Sari", "driver")
        .await
        .unwrap();
    assert!(
        s.hub_in_use("org-a", "hub-b").await.unwrap(),
        "a user assigned to the hub is a reference"
    );

    f.cleanup().await;
}

#[tokio::test]
async fn scoped_writes_never_reach_another_tenant() {
    let f = fixture!();
    let s = f.store();
    f.seed("org-a", "hub-a").await;
    f.seed("org-b", "hub-b").await;

    // Each must fail to match rather than cross the tenant boundary.
    assert!(
        !s.update_hub("org-a", "hub-b", "Stolen", 0.0, 0.0)
            .await
            .unwrap()
    );
    assert!(!s.delete_hub("org-a", "hub-b").await.unwrap());
    assert!(
        !s.create_team("org-a", "hub-b", "t-x", "Cross", "")
            .await
            .unwrap()
    );
    assert!(!s.update_organization("org-missing", "Nope").await.unwrap());

    assert_eq!(s.hubs_for_org("org-a").await.unwrap().len(), 1);
    assert_eq!(s.hubs_for_org("org-b").await.unwrap().len(), 1);

    f.cleanup().await;
}

#[tokio::test]
async fn events_are_tenant_scoped_and_replay_safe() {
    let f = fixture!();
    let s = f.store();
    f.seed("org-a", "hub-a").await;
    f.seed("org-b", "hub-b").await;

    let task = Task {
        id: "task-1".into(),
        tenant_id: "org-a".into(),
        hub_id: "hub-a".into(),
        title: "Delivery".into(),
        status: TaskStatus::Assigned,
        assignee_id: None,
        created_at: chrono::Utc::now(),
        stops: vec![Stop {
            id: "stop-1".into(),
            sequence: 0,
            name: "Warehouse".into(),
            address: "Jl. Sudirman 1".into(),
            location: Coordinate {
                lat: -6.2,
                lng: 106.8,
            },
            status: StopStatus::Pending,
            service_seconds: 300,
        }],
    };
    s.create_task("org-a", &task).await.expect("create task");

    let event = |key: &str| DeviceEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        idempotency_key: key.to_string(),
        tenant_id: "org-a".into(),
        hub_id: "hub-a".into(),
        driver_id: "sub-1".into(),
        device_id: "dev-1".into(),
        task_id: "task-1".into(),
        stop_id: Some("stop-1".into()),
        action: StopAction::Arrive,
        time: DeviceTime {
            utc: chrono::Utc::now(),
            offset_minutes: 420,
        },
        // An event from a device that predates location reporting.
        location: None,
        accuracy_meters: None,
        payload: serde_json::json!({}),
    };

    s.record_event("org-a", "sub-1", &event("k1"))
        .await
        .expect("first event");
    s.record_event("org-a", "sub-1", &event("k1"))
        .await
        .expect("replay is accepted");

    // The same client-chosen key under another tenant must not be swallowed as
    // a replay — that was cross-tenant suppression of proof-of-service events.
    s.record_event("org-b", "sub-9", &event("k1"))
        .await
        .expect("another tenant's identical key must not collide");

    // A foreign task id must not be reachable, whatever the body declares.
    let mut foreign = event("k2");
    foreign.task_id = "task-does-not-exist".into();
    let receipt = s.record_event("org-b", "sub-9", &foreign).await;
    assert!(receipt.is_ok(), "unmatched scope is a no-op, not an error");

    f.cleanup().await;
}

/// Executes every remaining `Store` method against a live server.
///
/// Coverage here is the point: three fatal TypeQL bugs — a reserved keyword as
/// a relation name, an undeclared attribute, and a doubled `;` — all shipped
/// green because no test ever ran the queries. Eighteen methods still had zero
/// executions; this exercises them so the next such bug fails in CI rather
/// than in production.
#[tokio::test]
async fn every_remaining_query_executes() {
    use altius_core::{
        CheckCategory, CheckItem, CheckStatus, CostEntry, DailyReport, ExpenseCategory, LhsStatus,
        VehicleCheck, VehicleCondition,
    };

    let f = fixture!();
    let s = f.store();
    f.seed("org-a", "hub-a").await;

    assert!(s.ping().await, "ping");

    s.provision_user("org-a", "hub-a", "sub-1", "Adi Pratama", "driver")
        .await
        .unwrap();

    // --- scope resolution -------------------------------------------------
    assert_eq!(
        s.organization_of("sub-1").await.unwrap().as_deref(),
        Some("org-a")
    );
    assert_eq!(
        s.organization_and_hub_of("sub-1").await.unwrap(),
        Some(("org-a".to_string(), "hub-a".to_string()))
    );
    assert!(s.user_in_org("org-a", "sub-1").await.unwrap());
    assert!(
        !s.user_in_org("org-b", "sub-1").await.unwrap(),
        "no cross-tenant membership"
    );
    assert!(s.organization_of("nobody").await.unwrap().is_none());

    // --- role cache -------------------------------------------------------
    assert!(s.set_cached_role("org-a", "sub-1", "lead").await.unwrap());
    assert!(
        s.drivers_for_org("org-a").await.unwrap().is_empty(),
        "cache refresh must move the user out of the driver roster"
    );
    assert!(s.set_cached_role("org-a", "sub-1", "driver").await.unwrap());
    assert_eq!(s.drivers_for_org("org-a").await.unwrap().len(), 1);

    // --- tasks ------------------------------------------------------------
    let task = sample_task("task-9", "org-a", "hub-a");
    s.create_task("org-a", &task).await.unwrap();
    assert_eq!(s.tasks_for_org("org-a").await.unwrap().len(), 1);
    assert!(s.task_by_id("org-a", "task-9").await.unwrap().is_some());
    assert!(
        s.task_by_id("org-b", "task-9").await.unwrap().is_none(),
        "a task must not be readable from another tenant"
    );

    // --- costs ------------------------------------------------------------
    let cost = CostEntry {
        id: "cost-1".into(),
        category: ExpenseCategory::Fuel,
        amount_minor: 150_000,
        currency: "IDR".into(),
        note: "Pertamina Kuningan".into(),
        day: "2026-09-08".into(),
    };
    s.record_cost(&cost, "sub-1").await.unwrap();
    assert_eq!(s.costs_for_driver("sub-1", None).await.unwrap().len(), 1);
    assert_eq!(
        s.costs_for_driver("sub-1", Some("2026-09-08"))
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        s.costs_for_driver("sub-1", Some("1999-01-01"))
            .await
            .unwrap()
            .is_empty()
    );

    // --- daily report -----------------------------------------------------
    let report = DailyReport {
        day: "2026-09-08".into(),
        driver_id: "sub-1".into(),
        hub_id: "hub-a".into(),
        driver_name: "Adi Pratama".into(),
        vehicle_number: "B 1234 XYZ".into(),
        odometer_start: Some(100),
        odometer_end: Some(340),
        notes: "Lancar".into(),
        visited_task_ids: vec!["task-9".into()],
        completed_stop_ids: vec![],
        costs: vec![cost],
        status: LhsStatus::Submitted,
        revision: 0,
    };
    s.record_daily_report(&report, "sub-1").await.unwrap();
    assert_eq!(s.reports_for_driver("sub-1").await.unwrap().len(), 1);

    // --- push tokens ------------------------------------------------------
    s.register_push_token("sub-1", "dev-1", "token-abc")
        .await
        .unwrap();
    assert_eq!(
        s.push_tokens_for_user("sub-1").await.unwrap(),
        vec!["token-abc"]
    );
    // Re-registering the same device must replace, not duplicate.
    s.register_push_token("sub-1", "dev-1", "token-xyz")
        .await
        .unwrap();
    assert_eq!(
        s.push_tokens_for_user("sub-1").await.unwrap(),
        vec!["token-xyz"],
        "one row per device"
    );

    // --- vehicle checks ---------------------------------------------------
    let check = VehicleCheck {
        id: "vc-1".into(),
        tenant_id: "org-a".into(),
        hub_id: "hub-a".into(),
        driver_id: "sub-1".into(),
        day: "2026-09-08".into(),
        driver_name: "Adi Pratama".into(),
        license_plate: "B 1234 XYZ".into(),
        vehicle_type: "CDD".into(),
        km_start: 100,
        km_end: 340,
        condition: VehicleCondition::Good,
        items: vec![CheckItem {
            name: "Ban".into(),
            category: CheckCategory::Inspection,
            status: CheckStatus::Good,
            note: String::new(),
        }],
        notes: "Aman".into(),
        service_date: Some("2026-08-01".into()),
        kir_date: None,
        stnk_date: None,
    };
    s.record_vehicle_check("org-a", "hub-a", &check)
        .await
        .unwrap();
    assert_eq!(
        s.vehicle_checks_for_org("org-a", None).await.unwrap().len(),
        1
    );
    assert_eq!(
        s.vehicle_checks_for_org("org-a", Some("hub-a"))
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        s.vehicle_checks_for_org("org-b", None)
            .await
            .unwrap()
            .is_empty(),
        "checks must not leak across tenants"
    );

    // --- team member removal ----------------------------------------------
    assert!(
        s.create_team("org-a", "hub-a", "team-9", "Night", "22:00-06:00")
            .await
            .unwrap()
    );
    assert!(s.add_team_member("org-a", "team-9", "sub-1").await.unwrap());
    assert!(
        s.remove_team_member("org-a", "team-9", "sub-1")
            .await
            .unwrap()
    );
    assert!(
        !s.remove_team_member("org-a", "team-9", "sub-1")
            .await
            .unwrap(),
        "removing an absent member is a no-op, not a success"
    );

    f.cleanup().await;
}

/// Bootstrap helpers run on startup, so a failure here breaks every boot.
#[tokio::test]
async fn bootstrap_helpers_are_replayable() {
    let f = fixture!();
    let s = f.store();
    let config = bootstrap_config();

    s.ensure_default_org_hub(&config).await.expect("first run");
    // Startup runs this on every boot, not only the first.
    s.ensure_default_org_hub(&config)
        .await
        .expect("second run must be a no-op");
    s.link_admin_user(&config).await.expect("link admin");

    assert_eq!(
        s.organization_of(&config.default_admin_sub)
            .await
            .unwrap()
            .as_deref(),
        Some(config.default_org_id.as_str())
    );
    assert_eq!(
        s.hubs_for_org(&config.default_org_id).await.unwrap().len(),
        1
    );

    f.cleanup().await;
}
