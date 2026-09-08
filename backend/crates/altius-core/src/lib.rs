//! Domain types mirroring `packages/api-contracts` Zod schemas 1:1.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type Id = String;

/// Workspace roles — match Keycloak realm roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
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
    /// JSON payload snapshot (proof drafts, notes).
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum EventReceipt {
    Queued,
    Sending,
    Accepted { server_event_id: Id },
    Conflict { reason: String },
    Rejected { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExpenseCategory {
    Fuel,
    Toll,
    Parking,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEntry {
    pub id: Id,
    pub category: ExpenseCategory,
    pub amount_minor: i64,
    pub currency: String,
    pub note: String,
    pub day: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LhsStatus {
    Draft,
    Submitted,
    RevisionRequested,
    Approved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
