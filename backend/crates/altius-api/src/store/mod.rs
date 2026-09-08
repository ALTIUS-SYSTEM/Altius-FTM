//! Persistence facade for the API.
//!
//! `Store` is the type the routes hold; it delegates to a concrete backend.
//! `Postgres` is the default transactional store. `Typedb` is kept for
//! relation/inference workloads that benefit from the graph model.

pub mod pg;
pub mod typedb;

use altius_core::{DeviceEvent, EventReceipt, Id, Task};
use serde_json::Value;

pub use pg::PgStore;
pub use typedb::TypedbStore;

/// Concrete backend selected at startup via `STORE_BACKEND`.
#[derive(Clone)]
pub enum Store {
    Postgres(PgStore),
    Typedb(TypedbStore),
}

impl Store {
    pub async fn organization_of(&self, subject: &str) -> anyhow::Result<Option<Id>> {
        match self {
            Self::Postgres(s) => s.organization_of(subject).await,
            Self::Typedb(s) => s.organization_of(subject).await,
        }
    }

    pub async fn tasks_for_org(&self, org_id: &str) -> anyhow::Result<Vec<Value>> {
        match self {
            Self::Postgres(s) => s.tasks_for_org(org_id).await,
            Self::Typedb(s) => s.tasks_for_org(org_id).await,
        }
    }

    pub async fn tasks_for_driver(
        &self,
        org_id: &str,
        driver_sub: &str,
    ) -> anyhow::Result<Vec<Value>> {
        match self {
            Self::Postgres(s) => s.tasks_for_driver(org_id, driver_sub).await,
            Self::Typedb(s) => s.tasks_for_driver(org_id, driver_sub).await,
        }
    }

    pub async fn task_by_id(&self, org_id: &str, task_id: &str) -> anyhow::Result<Option<Value>> {
        match self {
            Self::Postgres(s) => s.task_by_id(org_id, task_id).await,
            Self::Typedb(s) => s.task_by_id(org_id, task_id).await,
        }
    }

    pub async fn create_task(&self, org: &str, task: &Task) -> anyhow::Result<()> {
        match self {
            Self::Postgres(s) => s.create_task(org, task).await,
            Self::Typedb(s) => s.create_task(org, task).await,
        }
    }

    pub async fn update_task(&self, org: &str, task: &Task) -> anyhow::Result<()> {
        match self {
            Self::Postgres(s) => s.update_task(org, task).await,
            // TypeDB update path is not wired; refuse rather than silent no-op.
            Self::Typedb(_) => anyhow::bail!("task update is not supported on the TypeDB backend"),
        }
    }

    /// `None` — task not found; `Some(false)` — in progress, cancel first;
    /// `Some(true)` — deleted.
    pub async fn delete_task(&self, org: &str, task_id: &str) -> anyhow::Result<Option<bool>> {
        match self {
            Self::Postgres(s) => s.delete_task(org, task_id).await,
            Self::Typedb(_) => anyhow::bail!("task delete is not supported on the TypeDB backend"),
        }
    }

    pub async fn record_event(
        &self,
        org: &str,
        driver_sub: &str,
        ev: &DeviceEvent,
    ) -> anyhow::Result<EventReceipt> {
        match self {
            Self::Postgres(s) => s.record_event(org, driver_sub, ev).await,
            Self::Typedb(s) => s.record_event(org, driver_sub, ev).await,
        }
    }

    pub async fn ping(&self) -> bool {
        match self {
            Self::Postgres(s) => s.ping().await,
            Self::Typedb(s) => s.ping().await,
        }
    }

    pub async fn ensure_default_org_hub(
        &self,
        config: &crate::config::Config,
    ) -> anyhow::Result<()> {
        match self {
            Self::Postgres(s) => s.ensure_default_org_hub(config).await,
            Self::Typedb(s) => s.ensure_default_org_hub(config).await,
        }
    }

    pub async fn link_admin_user(&self, config: &crate::config::Config) -> anyhow::Result<()> {
        match self {
            Self::Postgres(s) => s.link_admin_user(config).await,
            Self::Typedb(s) => s.link_admin_user(config).await,
        }
    }

    pub async fn provision_user(
        &self,
        org: &str,
        hub: &str,
        subject: &str,
        display_name: &str,
        role: &str,
    ) -> anyhow::Result<()> {
        match self {
            Self::Postgres(s) => {
                s.provision_user(org, hub, subject, display_name, role)
                    .await
            }
            Self::Typedb(s) => {
                s.provision_user(org, hub, subject, display_name, role)
                    .await
            }
        }
    }

    pub async fn create_hub(
        &self,
        org: &str,
        hub_id: &str,
        name: &str,
        lat: f64,
        lng: f64,
    ) -> anyhow::Result<bool> {
        match self {
            Self::Postgres(s) => s.create_hub(org, hub_id, name, lat, lng).await,
            Self::Typedb(s) => s.create_hub(org, hub_id, name, lat, lng).await,
        }
    }

    pub async fn update_hub(
        &self,
        org: &str,
        hub_id: &str,
        name: &str,
        lat: f64,
        lng: f64,
    ) -> anyhow::Result<bool> {
        match self {
            Self::Postgres(s) => s.update_hub(org, hub_id, name, lat, lng).await,
            Self::Typedb(s) => s.update_hub(org, hub_id, name, lat, lng).await,
        }
    }

    pub async fn hub_in_use(&self, org: &str, hub_id: &str) -> anyhow::Result<bool> {
        match self {
            Self::Postgres(s) => s.hub_in_use(org, hub_id).await,
            Self::Typedb(s) => s.hub_in_use(org, hub_id).await,
        }
    }

    pub async fn delete_hub(&self, org: &str, hub_id: &str) -> anyhow::Result<bool> {
        match self {
            Self::Postgres(s) => s.delete_hub(org, hub_id).await,
            Self::Typedb(s) => s.delete_hub(org, hub_id).await,
        }
    }

    pub async fn update_organization(&self, org: &str, name: &str) -> anyhow::Result<bool> {
        match self {
            Self::Postgres(s) => s.update_organization(org, name).await,
            Self::Typedb(s) => s.update_organization(org, name).await,
        }
    }

    pub async fn teams_for_org(&self, org: &str) -> anyhow::Result<Vec<Value>> {
        match self {
            Self::Postgres(s) => s.teams_for_org(org).await,
            Self::Typedb(s) => s.teams_for_org(org).await,
        }
    }

    pub async fn create_team(
        &self,
        org: &str,
        hub_id: &str,
        team_id: &str,
        name: &str,
        shift: &str,
    ) -> anyhow::Result<bool> {
        match self {
            Self::Postgres(s) => s.create_team(org, hub_id, team_id, name, shift).await,
            Self::Typedb(s) => s.create_team(org, hub_id, team_id, name, shift).await,
        }
    }

    pub async fn update_team(
        &self,
        org: &str,
        team_id: &str,
        name: &str,
        shift: &str,
    ) -> anyhow::Result<bool> {
        match self {
            Self::Postgres(s) => s.update_team(org, team_id, name, shift).await,
            Self::Typedb(s) => s.update_team(org, team_id, name, shift).await,
        }
    }

    pub async fn delete_team(&self, org: &str, team_id: &str) -> anyhow::Result<bool> {
        match self {
            Self::Postgres(s) => s.delete_team(org, team_id).await,
            Self::Typedb(s) => s.delete_team(org, team_id).await,
        }
    }

    pub async fn add_team_member(
        &self,
        org: &str,
        team_id: &str,
        subject: &str,
    ) -> anyhow::Result<bool> {
        match self {
            Self::Postgres(s) => s.add_team_member(org, team_id, subject).await,
            Self::Typedb(s) => s.add_team_member(org, team_id, subject).await,
        }
    }

    pub async fn remove_team_member(
        &self,
        org: &str,
        team_id: &str,
        subject: &str,
    ) -> anyhow::Result<bool> {
        match self {
            Self::Postgres(s) => s.remove_team_member(org, team_id, subject).await,
            Self::Typedb(s) => s.remove_team_member(org, team_id, subject).await,
        }
    }

    pub async fn set_cached_role(
        &self,
        org: &str,
        subject: &str,
        role: &str,
    ) -> anyhow::Result<bool> {
        match self {
            Self::Postgres(s) => s.set_cached_role(org, subject, role).await,
            Self::Typedb(s) => s.set_cached_role(org, subject, role).await,
        }
    }

    pub async fn user_in_org(&self, org: &str, subject: &str) -> anyhow::Result<bool> {
        match self {
            Self::Postgres(s) => s.user_in_org(org, subject).await,
            Self::Typedb(s) => s.user_in_org(org, subject).await,
        }
    }

    pub async fn organization_and_hub_of(&self, subject: &str) -> anyhow::Result<Option<(Id, Id)>> {
        match self {
            Self::Postgres(s) => s.organization_and_hub_of(subject).await,
            Self::Typedb(s) => s.organization_and_hub_of(subject).await,
        }
    }

    pub async fn register_push_token(
        &self,
        subject: &str,
        device_id: &str,
        token: &str,
    ) -> anyhow::Result<()> {
        match self {
            Self::Postgres(s) => s.register_push_token(subject, device_id, token).await,
            Self::Typedb(s) => s.register_push_token(subject, device_id, token).await,
        }
    }

    pub async fn push_tokens_for_user(&self, subject: &str) -> anyhow::Result<Vec<String>> {
        match self {
            Self::Postgres(s) => s.push_tokens_for_user(subject).await,
            Self::Typedb(s) => s.push_tokens_for_user(subject).await,
        }
    }

    pub async fn users_for_org(&self, org_id: &str) -> anyhow::Result<Vec<Value>> {
        match self {
            Self::Postgres(s) => s.users_for_org(org_id).await,
            Self::Typedb(s) => s.users_for_org(org_id).await,
        }
    }

    pub async fn hubs_for_org(&self, org_id: &str) -> anyhow::Result<Vec<Value>> {
        match self {
            Self::Postgres(s) => s.hubs_for_org(org_id).await,
            Self::Typedb(s) => s.hubs_for_org(org_id).await,
        }
    }

    pub async fn drivers_for_org(&self, org_id: &str) -> anyhow::Result<Vec<Value>> {
        match self {
            Self::Postgres(s) => s.drivers_for_org(org_id).await,
            Self::Typedb(s) => s.drivers_for_org(org_id).await,
        }
    }

    pub async fn record_cost(
        &self,
        org: &str,
        hub: &str,
        entry: &altius_core::CostEntry,
        driver_sub: &str,
    ) -> anyhow::Result<()> {
        match self {
            Self::Postgres(s) => s.record_cost(org, hub, entry, driver_sub).await,
            Self::Typedb(s) => s.record_cost(org, hub, entry, driver_sub).await,
        }
    }

    pub async fn costs_for_driver(
        &self,
        org: &str,
        driver_sub: &str,
        day: Option<&str>,
    ) -> anyhow::Result<Vec<Value>> {
        match self {
            Self::Postgres(s) => s.costs_for_driver(org, driver_sub, day).await,
            Self::Typedb(s) => s.costs_for_driver(org, driver_sub, day).await,
        }
    }

    pub async fn record_daily_report(
        &self,
        org: &str,
        report: &altius_core::DailyReport,
        driver_sub: &str,
    ) -> anyhow::Result<()> {
        match self {
            Self::Postgres(s) => s.record_daily_report(org, report, driver_sub).await,
            Self::Typedb(s) => s.record_daily_report(org, report, driver_sub).await,
        }
    }

    pub async fn reports_for_driver(&self, org: &str, driver_sub: &str) -> anyhow::Result<Vec<Value>> {
        match self {
            Self::Postgres(s) => s.reports_for_driver(org, driver_sub).await,
            Self::Typedb(s) => s.reports_for_driver(org, driver_sub).await,
        }
    }

    pub async fn record_vehicle_check(
        &self,
        org: &str,
        hub: &str,
        check: &altius_core::VehicleCheck,
    ) -> anyhow::Result<()> {
        match self {
            Self::Postgres(s) => s.record_vehicle_check(org, hub, check).await,
            Self::Typedb(s) => s.record_vehicle_check(org, hub, check).await,
        }
    }

    pub async fn vehicle_checks_for_org(
        &self,
        org: &str,
        hub: Option<&str>,
    ) -> anyhow::Result<Vec<Value>> {
        match self {
            Self::Postgres(s) => s.vehicle_checks_for_org(org, hub).await,
            Self::Typedb(s) => s.vehicle_checks_for_org(org, hub).await,
        }
    }

    pub async fn record_gps_observation(
        &self,
        obs: &altius_core::GpsObservation,
    ) -> anyhow::Result<()> {
        match self {
            Self::Postgres(s) => s.record_gps_observation(obs).await,
            Self::Typedb(s) => s.record_gps_observation(obs).await,
        }
    }

    pub async fn record_gps_review(&self, review: &altius_core::GpsReview) -> anyhow::Result<()> {
        match self {
            Self::Postgres(s) => s.record_gps_review(review).await,
            Self::Typedb(s) => s.record_gps_review(review).await,
        }
    }

    pub async fn mark_gps_review_reviewed(
        &self,
        org: &str,
        review_id: &str,
        reviewer: &str,
    ) -> anyhow::Result<bool> {
        match self {
            Self::Postgres(s) => s.mark_gps_review_reviewed(org, review_id, reviewer).await,
            Self::Typedb(s) => s.mark_gps_review_reviewed(org, review_id, reviewer).await,
        }
    }

    pub async fn gps_observations_for_org(
        &self,
        org: &str,
        source: Option<&str>,
        since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> anyhow::Result<Vec<Value>> {
        match self {
            Self::Postgres(s) => s.gps_observations_for_org(org, source, since).await,
            Self::Typedb(s) => s.gps_observations_for_org(org, source, since).await,
        }
    }

    pub async fn gps_reviews_for_org(&self, org: &str) -> anyhow::Result<Vec<Value>> {
        match self {
            Self::Postgres(s) => s.gps_reviews_for_org(org).await,
            Self::Typedb(s) => s.gps_reviews_for_org(org).await,
        }
    }

    pub async fn monitoring_vehicles(&self, org: &str) -> anyhow::Result<Vec<Value>> {
        match self {
            Self::Postgres(s) => s.monitoring_vehicles(org).await,
            Self::Typedb(s) => s.monitoring_vehicles(org).await,
        }
    }

    pub async fn organizations(&self) -> anyhow::Result<Vec<String>> {
        match self {
            Self::Postgres(s) => s.organizations().await,
            Self::Typedb(s) => s.organizations().await,
        }
    }

    pub async fn mceasy_sync_vehicle(
        &self,
        org: &str,
        plate: &str,
        mceasy_id: &str,
    ) -> anyhow::Result<()> {
        match self {
            Self::Postgres(s) => s.mceasy_sync_vehicle(org, plate, mceasy_id).await,
            Self::Typedb(s) => s.mceasy_sync_vehicle(org, plate, mceasy_id).await,
        }
    }

    pub async fn mceasy_sync_driver(
        &self,
        org: &str,
        user_sub: &str,
        mceasy_id: &str,
    ) -> anyhow::Result<()> {
        match self {
            Self::Postgres(s) => s.mceasy_sync_driver(org, user_sub, mceasy_id).await,
            Self::Typedb(s) => s.mceasy_sync_driver(org, user_sub, mceasy_id).await,
        }
    }

    pub async fn prune_gps_observations_older_than(
        &self,
        boundary: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<()> {
        match self {
            Self::Postgres(s) => s.prune_gps_observations_older_than(boundary).await,
            Self::Typedb(s) => s.prune_gps_observations_older_than(boundary).await,
        }
    }
}
