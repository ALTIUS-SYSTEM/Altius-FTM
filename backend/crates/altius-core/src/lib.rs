//! Domain types mirroring `packages/api-contracts` Zod schemas 1:1.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod gps;

pub type Id = String;

/// Workspace roles — match Keycloak realm roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    SuperAdmin,
    Admin,
    Supervisor,
    Lead,
    Driver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Unassigned,
    Assigned,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopStatus {
    Pending,
    Arrived,
    Working,
    Completed,
    Departed,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopAction {
    Arrive,
    StartActivity,
    CompleteActivity,
    Depart,
    Skip,
}

impl StopAction {
    /// Next legal status for this action, mirroring the mobile state machine.
    pub fn target(self) -> StopStatus {
        match self {
            Self::Arrive => StopStatus::Arrived,
            Self::StartActivity => StopStatus::Working,
            Self::CompleteActivity => StopStatus::Completed,
            Self::Depart => StopStatus::Departed,
            Self::Skip => StopStatus::Skipped,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coordinate {
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stop {
    pub id: Id,
    pub sequence: u32,
    pub name: String,
    pub address: String,
    pub location: Coordinate,
    pub status: StopStatus,
    pub service_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: Id,
    pub tenant_id: Id,
    pub hub_id: Id,
    pub title: String,
    pub status: TaskStatus,
    pub assignee_id: Option<Id>,
    pub stops: Vec<Stop>,
    pub created_at: DateTime<Utc>,
}

/// Device clock with explicit UTC instant and bounded zone offset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTime {
    pub utc: DateTime<Utc>,
    /// Minutes east of UTC; |offset| <= 840 (14h).
    pub offset_minutes: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceEvent {
    pub event_id: Id,
    pub idempotency_key: Id,
    pub tenant_id: Id,
    pub hub_id: Id,
    pub driver_id: Id,
    pub device_id: Id,
    pub task_id: Id,
    pub stop_id: Option<Id>,
    pub action: StopAction,
    pub time: DeviceTime,
    /// Optional app GPS fix captured when the event was created. Used for
    /// comparing the driver's app position with the vehicle telematics stream.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<Coordinate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accuracy_meters: Option<f64>,
    /// JSON payload snapshot (proof drafts, notes). Deliberately untyped: no
    /// Zod counterpart constrains this on the TS side either (see
    /// VULN-FINDINGS.md F-07-19) — callers must not trust its shape.
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum EventReceipt {
    Queued,
    Sending,
    /// `event_id` is the client id so receipts can be matched without relying on
    /// batch order; `server_event_id` is the persisted row id (same on first
    /// accept, same on idempotent replay).
    Accepted {
        event_id: Id,
        server_event_id: Id,
    },
    Conflict {
        event_id: Id,
        reason: String,
    },
    Rejected {
        event_id: Id,
        reason: String,
    },
}

// Must stay 1:1 with `ExpenseCategorySchema` in packages/api-contracts/src/lhs.ts
// (fuel, toll, parking, meal, maintenance, other) — this enum previously had
// only 4 of the 6 variants, so `meal`/`maintenance` from any client following
// the TS contract would fail to deserialize here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExpenseCategory {
    Fuel,
    Toll,
    Parking,
    Meal,
    Maintenance,
    Other,
}

/// 10^12 minor units, mirroring `MAX_AMOUNT_MINOR` in
/// packages/api-contracts/src/primitives.ts — keep the two in sync.
pub const MAX_AMOUNT_MINOR: i64 = 1_000_000_000_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CostEntry {
    pub id: Id,
    pub category: ExpenseCategory,
    pub amount_minor: i64,
    pub currency: String,
    pub note: String,
    pub day: String,
}

impl CostEntry {
    /// Mirrors the Zod `MoneySchema` bound: non-negative and capped at
    /// `MAX_AMOUNT_MINOR`, plus a bare 3-letter currency code. The Rust side
    /// previously accepted any `i64` (including negative) and any string.
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.amount_minor < 0 || self.amount_minor > MAX_AMOUNT_MINOR {
            return Err(CoreError::Validation(format!(
                "amount_minor must be within 0..={MAX_AMOUNT_MINOR}"
            )));
        }
        if self.currency.len() != 3 || !self.currency.chars().all(|c| c.is_ascii_uppercase()) {
            return Err(CoreError::Validation(
                "currency must be a 3-letter uppercase code".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LhsStatus {
    Draft,
    Submitted,
    RevisionRequested,
    Approved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckCategory {
    Equipment,
    Inspection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Good,
    Damaged,
    Present,
    Missing,
    Na,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VehicleCondition {
    Good,
    NotGood,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckItem {
    pub name: String,
    pub category: CheckCategory,
    pub status: CheckStatus,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleCheck {
    pub id: Id,
    pub tenant_id: Id,
    pub hub_id: Id,
    pub driver_id: Id,
    pub day: String,
    pub driver_name: String,
    pub license_plate: String,
    pub vehicle_type: String,
    pub km_start: u32,
    pub km_end: u32,
    pub condition: VehicleCondition,
    pub items: Vec<CheckItem>,
    pub notes: String,
    pub service_date: Option<String>,
    pub kir_date: Option<String>,
    pub stnk_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DailyReport {
    pub day: String,
    pub driver_id: Id,
    pub hub_id: Id,
    pub driver_name: String,
    pub vehicle_number: String,
    pub odometer_start: Option<u32>,
    pub odometer_end: Option<u32>,
    pub notes: String,
    pub visited_task_ids: Vec<Id>,
    pub completed_stop_ids: Vec<Id>,
    pub costs: Vec<CostEntry>,
    pub status: LhsStatus,
    pub revision: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpsSource {
    AppGps,
    VehicleGps,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpsQuality {
    Accurate,
    Degraded,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GpsObservation {
    pub id: Id,
    pub tenant_id: Id,
    pub hub_id: Id,
    pub driver_id: Id,
    pub vehicle_id: Option<Id>,
    pub time: DeviceTime,
    pub source: GpsSource,
    pub quality: GpsQuality,
    pub location: Option<Coordinate>,
    pub accuracy_meters: Option<f64>,
    pub speed_mps: Option<f64>,
    pub mock_location_reported: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpsReviewClassification {
    Consistent,
    ReviewRequired,
    InsufficientData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpsReviewReason {
    WithinTolerance,
    Separation,
    MissingPair,
    PoorAccuracy,
    Stale,
    ScopeMismatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GpsReview {
    pub id: Id,
    pub tenant_id: Id,
    pub hub_id: Id,
    pub driver_id: Id,
    pub vehicle_id: Option<Id>,
    pub app_observation_id: Id,
    pub vehicle_observation_id: Option<Id>,
    pub classification: GpsReviewClassification,
    pub separation_meters: Option<f64>,
    pub time_delta_seconds: Option<f64>,
    pub reason: GpsReviewReason,
    pub reviewed_by: Option<Id>,
    pub created_at: DateTime<Utc>,
}

impl GpsObservation {
    pub fn day(&self) -> String {
        self.time.utc.date_naive().to_string()
    }
}

impl GpsReview {
    pub fn day(&self) -> String {
        self.created_at.date_naive().to_string()
    }
}

/// Authenticated principal resolved from a Keycloak token.
#[derive(Debug, Clone)]
pub struct Principal {
    pub subject: Id,
    pub roles: Vec<Role>,
    pub organization_id: Option<Id>,
}

impl Principal {
    pub fn has_role(&self, role: Role) -> bool {
        self.roles.contains(&role)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("invalid stop transition: {from:?} -> {to:?}")]
    InvalidTransition { from: StopStatus, to: StopStatus },
    #[error("validation failed: {0}")]
    Validation(String),
}

/// Sequential stage enforcement, identical to `WorkStore` on mobile.
pub fn check_transition(from: StopStatus, action: StopAction) -> Result<StopStatus, CoreError> {
    let to = action.target();
    let legal = matches!(
        (from, to),
        (StopStatus::Pending, StopStatus::Arrived)
            | (StopStatus::Arrived, StopStatus::Working)
            | (StopStatus::Working, StopStatus::Completed)
            | (StopStatus::Completed, StopStatus::Departed)
            | (StopStatus::Pending, StopStatus::Skipped)
    );
    if legal {
        Ok(to)
    } else {
        Err(CoreError::InvalidTransition { from, to })
    }
}

pub fn new_id() -> Id {
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests;
