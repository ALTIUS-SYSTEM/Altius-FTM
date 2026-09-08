//! TypeDB persistence — atomic event+task writes mirroring the mobile
//! `WorkStore` guarantees (single transaction, immutable events, request-key
//! idempotency).

use anyhow::Context;
use futures::StreamExt;
use serde_json::Value;
use typedb_driver::answer::QueryAnswer;
use typedb_driver::{TransactionType, TypeDBDriver};

use altius_core::{check_transition, DeviceEvent, EventReceipt, Id, StopStatus, Task};

pub struct Store {
    driver: TypeDBDriver,
    database: String,
}

impl Store {
    pub fn new(driver: TypeDBDriver, database: String) -> Self {
        Self { driver, database }
    }

    /// Escape a value for embedding in a double-quoted TypeQL string literal.
    /// Backslash and quote are escaped; C0 control characters are escaped or
    /// dropped so they can never terminate the literal and let `;`/`#` in the
    /// remainder be parsed as query syntax.
    fn esc(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '\\' => out.push_str("\\\\"),
                '"' => out.push_str("\\\""),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                // Remaining C0 + DEL have no escape form and no legitimate use
                // in these fields; drop them rather than emit a raw byte.
                c if c.is_control() => {}
                c => out.push(c),
            }
        }
        out
    }

    /// Namespace a client-chosen idempotency key by tenant and driver so one
    /// caller cannot burn (and thereby silently suppress) another caller's key.
    /// `request-key` is `@unique` globally in the schema.
    /// Length-prefixed so the parts cannot be re-partitioned: org "a"/driver
    /// "bc" and org "ab"/driver "c" must not collide. A control-character
    /// separator would not survive `esc`, which strips C0.
    fn request_key(org: &str, driver_sub: &str, key: &str) -> String {
        format!(
            "{}:{org}|{}:{driver_sub}|{key}",
            org.len(),
            driver_sub.len()
        )
    }

    async fn fetch_all(&self, query: &str) -> anyhow::Result<Vec<Value>> {
        let tx = self
            .driver
            .transaction(&self.database, TransactionType::Read)
            .await?;
        let answer = tx.query(query).await?;
        let mut out = Vec::new();
        if let QueryAnswer::ConceptDocumentStream(_, mut docs) = answer {
            while let Some(doc) = docs.next().await {
                let doc = doc?;
                out.push(serde_json::to_value(doc.into_json())?);
            }
        }
        tx.close().await.ok();
        Ok(out)
    }

    /// Resolve the organization bound to a principal subject (tenant scoping).
    pub async fn organization_of(&self, subject: &str) -> anyhow::Result<Option<Id>> {
        let q = format!(
            r#"match
                $u isa user, has user-sub "{}";
                membership (member: $u, org: $o);
                $o has org-id $oid;
            fetch {{ "org": $oid }};"#,
            Self::esc(subject)
        );
        let rows = self.fetch_all(&q).await?;
        Ok(rows
            .first()
            .and_then(|d| d.get("org"))
            .and_then(|v| v.as_str())
            .map(str::to_string))
    }

    /// Tasks visible to an organization, with their stops embedded.
    pub async fn tasks_for_org(&self, org_id: &str) -> anyhow::Result<Vec<Value>> {
        let q = format!(
            r#"match
                allocation (org: $o, hub: $h);
                $o has org-id "{org}";
                located (task: $t, hub: $h);
            fetch {{
                "task": {{ $t.* }},
                "stops": [
                    match contains (parent: $t, child: $s);
                    fetch {{ "stop": {{ $s.* }} }};
                ]
            }};"#,
            org = Self::esc(org_id)
        );
        self.fetch_all(&q).await
    }

    pub async fn task_by_id(&self, org_id: &str, task_id: &str) -> anyhow::Result<Option<Value>> {
        let q = format!(
            r#"match
                allocation (org: $o, hub: $h);
                $o has org-id "{org}";
                located (task: $t, hub: $h);
                $t has task-id "{task}";
            fetch {{
                "task": {{ $t.* }},
                "stops": [
                    match contains (parent: $t, child: $s);
                    fetch {{ "stop": {{ $s.* }} }};
                ]
            }};"#,
            org = Self::esc(org_id),
            task = Self::esc(task_id)
        );
        Ok(self.fetch_all(&q).await?.into_iter().next())
    }

    /// Upsert org → hub → task → stops in one write transaction.
    ///
    /// `org` is the caller's server-resolved organization, never a body field:
    /// the hub must be allocated to it, so a task can only ever land in a hub
    /// the authenticated caller's tenant owns.
    pub async fn create_task(&self, org: &str, task: &Task) -> anyhow::Result<()> {
        let tx = self
            .driver
            .transaction(&self.database, TransactionType::Write)
            .await
            .context("open write tx")?;

        // Confirm the hub is allocated to the caller's org before inserting.
        // Without this an empty `match` would make the `insert` a silent no-op
        // that still reports success to the client.
        let scope = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                $h isa hub, has hub-id "{hub}";
                allocation (org: $o, hub: $h);
            select $h;"#,
            org = Self::esc(org),
            hub = Self::esc(&task.hub_id),
        );
        let allocated = match tx.query(&scope).await.context("verify hub allocation")? {
            QueryAnswer::ConceptRowStream(_, mut rows) => rows.next().await.is_some(),
            _ => false,
        };
        if !allocated {
            tx.rollback().await.ok();
            anyhow::bail!("hub {} is not allocated to organization {org}", task.hub_id);
        }

        let stage = serde_json::to_value(task.status)?
            .as_str()
            .unwrap_or("assigned")
            .to_string();
        let mut q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                $h isa hub, has hub-id "{hub}";
                allocation (org: $o, hub: $h);
            insert
                $t isa task,
                    has task-id "{tid}",
                    has title "{title}",
                    has stage "{stage}",
                    has day "{day}";
                located (task: $t, hub: $h);"#,
            org = Self::esc(org),
            hub = Self::esc(&task.hub_id),
            tid = Self::esc(&task.id),
            title = Self::esc(&task.title),
            stage = Self::esc(&stage),
            day = Self::esc(&task.created_at.date_naive().to_string()),
        );

        for stop in &task.stops {
            let sstage = serde_json::to_value(stop.status)?
                .as_str()
                .unwrap_or("pending")
                .to_string();
            q.push_str(&format!(
                r#"
                $s{n} isa stop,
                    has stop-id "{sid}",
                    has task-id "{tid}",
                    has sequence {seq},
                    has display-name "{name}",
                    has address "{addr}",
                    has latitude {lat},
                    has longitude {lng},
                    has stage "{sstage}";
                contains (parent: $t, child: $s{n});"#,
                n = stop.sequence,
                sid = Self::esc(&stop.id),
                tid = Self::esc(&task.id),
                seq = stop.sequence,
                name = Self::esc(&stop.name),
                addr = Self::esc(&stop.address),
                lat = stop.location.lat,
                lng = stop.location.lng,
                sstage = Self::esc(&sstage),
            ));
        }
        q.push(';');
        tx.query(&q).await.context("insert task")?;
        tx.commit().await.context("commit")?;
        Ok(())
    }

    /// Append a device event atomically: insert the immutable event, link it
    /// to the task (`recorded`) and stop (`reports`), then advance the stop
    /// and task stages — all in one write transaction.
    /// Replayed request keys short-circuit to `Accepted` without re-writing.
    ///
    /// `org` and `driver_sub` are server-resolved from the caller's token. Every
    /// task and stop lookup is constrained by `org`, so a client cannot reach
    /// another tenant's rows by supplying their ids; the idempotency key is
    /// namespaced by both, so one tenant cannot burn another tenant's keys.
    pub async fn record_event(
        &self,
        org: &str,
        driver_sub: &str,
        ev: &DeviceEvent,
    ) -> anyhow::Result<EventReceipt> {
        let tx = self
            .driver
            .transaction(&self.database, TransactionType::Write)
            .await
            .context("open write tx")?;

        let request_key = Self::request_key(org, driver_sub, &ev.idempotency_key);
        let check = format!(
            r#"match $e isa device-event, has request-key "{}", has event-id $eid;
            fetch {{ "event": $eid }};"#,
            Self::esc(&request_key)
        );
        if let QueryAnswer::ConceptDocumentStream(_, mut docs) =
            tx.query(&check).await.context("idempotency check")?
            && let Some(doc) = docs.next().await
        {
            let v = serde_json::to_value(doc?.into_json())?;
            tx.rollback().await.ok();
            // Return the *stored* event id, not the client's key, so both the
            // replay and the fresh path yield the same server-side identifier.
            return Ok(EventReceipt::Accepted {
                server_event_id: v
                    .get("event")
                    .and_then(|s| s.as_str())
                    .unwrap_or(&ev.event_id)
                    .to_string(),
            });
        }

        // Read current stop stage to enforce the transition rules. The stop must
        // hang off a task located in a hub allocated to the caller's org.
        let mut new_stage: Option<StopStatus> = None;
        if let Some(stop_id) = &ev.stop_id {
            let stage_q = format!(
                r#"match
                    $o isa organization, has org-id "{org}";
                    allocation (org: $o, hub: $h);
                    located (task: $t, hub: $h);
                    $t has task-id "{tid}";
                    contains (parent: $t, child: $s);
                    $s isa stop, has stop-id "{sid}", has stage $st;
                fetch {{ "stage": $st }};"#,
                org = Self::esc(org),
                tid = Self::esc(&ev.task_id),
                sid = Self::esc(stop_id),
            );
            let mut resolved = false;
            if let QueryAnswer::ConceptDocumentStream(_, mut docs) =
                tx.query(&stage_q).await.context("read stop stage")?
                && let Some(doc) = docs.next().await {
                    let v = serde_json::to_value(doc?.into_json())?;
                    if let Some(cur) = v.get("stage").and_then(|s| s.as_str()) {
                        resolved = true;
                        let Ok(from) = serde_json::from_str::<StopStatus>(
                            &format!("\"{cur}\""),
                        ) else {
                            tx.rollback().await.ok();
                            return Ok(EventReceipt::Conflict {
                                reason: format!("stop {stop_id} has unknown stage {cur:?}"),
                            });
                        };
                        match check_transition(from, ev.action) {
                            Ok(next) => new_stage = Some(next),
                            Err(e) => {
                                tx.rollback().await.ok();
                                return Ok(EventReceipt::Conflict {
                                    reason: e.to_string(),
                                });
                            }
                        }
                    }
                }
            // No match means the stop does not exist, does not belong to this
            // task, or the task is outside the caller's org. Reject rather than
            // silently recording an event that advances nothing.
            if !resolved {
                tx.rollback().await.ok();
                return Ok(EventReceipt::Conflict {
                    reason: format!("stop {stop_id} is not part of task {} in this organization", ev.task_id),
                });
            }
        }

        let insert = format!(
            r#"insert $e isa device-event,
                has event-id "{id}",
                has request-key "{key}",
                has action "{action}",
                has occurred-utc {utc},
                has offset-min {off},
                has day "{day}",
                has payload {payload};
            match
                $o isa organization, has org-id "{org}";
                allocation (org: $o, hub: $h);
                located (task: $t, hub: $h);
                $t has task-id "{tid}";
            insert
                recorded (event: $e, task: $t);"#,
            id = Self::esc(&ev.event_id),
            key = Self::esc(&request_key),
            org = Self::esc(org),
            action = Self::esc(&format!("{:?}", ev.action).to_lowercase()),
            utc = ev.time.utc.format("%Y-%m-%dT%H:%M:%S%.3f+00:00"),
            off = ev.time.offset_minutes,
            day = Self::esc(&ev.time.utc.date_naive().to_string()),
            payload = serde_json::to_string(&ev.payload.to_string())?,
            tid = Self::esc(&ev.task_id),
        );
        tx.query(&insert).await.context("insert event")?;

        // Link event to its stop and advance the stop stage.
        if let (Some(stop_id), Some(stage)) = (&ev.stop_id, new_stage) {
            let stage_name = serde_json::to_value(stage)?
                .as_str()
                .unwrap_or_default()
                .to_string();
            let link = format!(
                r#"match
                    $e isa device-event, has event-id "{eid}";
                    $o isa organization, has org-id "{org}";
                    allocation (org: $o, hub: $h);
                    located (task: $t, hub: $h);
                    $t has task-id "{tid}";
                    contains (parent: $t, child: $s);
                    $s isa stop, has stop-id "{sid}", has stage $old;
                delete has $old of $s;
                insert
                    $s has stage "{stage}";
                    reports (stop: $s, event: $e);"#,
                eid = Self::esc(&ev.event_id),
                org = Self::esc(org),
                tid = Self::esc(&ev.task_id),
                sid = Self::esc(stop_id),
                stage = Self::esc(&stage_name),
            );
            tx.query(&link).await.context("link stop + advance")?;
        }

        // Roll the task stage forward when a stop departs.
        if matches!(new_stage, Some(StopStatus::Departed)) {
            let roll = format!(
                r#"match
                    $o isa organization, has org-id "{org}";
                    allocation (org: $o, hub: $h);
                    located (task: $t, hub: $h);
                    $t has task-id "{tid}", has stage $old;
                delete has $old of $t;
                insert $t has stage "in_progress";"#,
                org = Self::esc(org),
                tid = Self::esc(&ev.task_id),
            );
            tx.query(&roll).await.context("advance task stage")?;
        }

        // The check-then-insert above is not atomic across concurrent
        // transactions; `request-key @unique` is the real guard and only fires
        // here. A uniqueness violation means someone else committed the same
        // key first — which is exactly the replay this endpoint promises to
        // absorb, so report it as accepted rather than 500-ing the whole batch.
        if let Err(e) = tx.commit().await {
            let text = e.to_string();
            if text.contains("request-key") || text.to_lowercase().contains("unique") {
                return Ok(EventReceipt::Accepted {
                    server_event_id: ev.event_id.clone(),
                });
            }
            return Err(anyhow::Error::new(e).context("commit"));
        }
        Ok(EventReceipt::Accepted {
            server_event_id: ev.event_id.clone(),
        })
    }

    /// Health probe for the database.
    pub async fn ping(&self) -> bool {
        match self
            .driver
            .transaction(&self.database, TransactionType::Read)
            .await
        {
            Ok(tx) => {
                tx.close().await.ok();
                true
            }
            Err(_) => false,
        }
    }

    /// Insert the default organization and hub if they do not exist.
    pub async fn ensure_default_org_hub(&self, config: &crate::config::Config) -> anyhow::Result<()> {
        let tx = self
            .driver
            .transaction(&self.database, TransactionType::Write)
            .await
            .context("open write tx for provisioning")?;
        let q = format!(
            r#"insert
                $o isa organization, has org-id "{org}", has display-name "{org_name}";
                $h isa hub, has hub-id "{hub}", has display-name "{hub_name}";
                allocation (org: $o, hub: $h);"#,
            org = Self::esc(&config.default_org_id),
            org_name = Self::esc(&config.default_org_name),
            hub = Self::esc(&config.default_hub_id),
            hub_name = Self::esc(&config.default_hub_name),
        );
        // TypeDB ignores inserts that violate key uniqueness, so the statement is safe to replay.
        tx.query(&q).await.context("insert default org/hub")?;
        tx.commit().await.context("commit default org/hub")?;
        Ok(())
    }

    /// Link the configured default admin subject to the default organization.
    pub async fn link_admin_user(&self, config: &crate::config::Config) -> anyhow::Result<()> {
        let tx = self
            .driver
            .transaction(&self.database, TransactionType::Write)
            .await
            .context("open write tx for admin link")?;
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
            insert
                $u isa user, has user-sub "{sub}";
                membership (member: $u, org: $o);"#,
            org = Self::esc(&config.default_org_id),
            sub = Self::esc(&config.default_admin_sub),
        );
        tx.query(&q).await.context("link admin user")?;
        tx.commit().await.context("commit admin link")?;
        Ok(())
    }

    /// Resolve the organization and first hub bound to a principal subject.
    pub async fn organization_and_hub_of(
        &self,
        subject: &str,
    ) -> anyhow::Result<Option<(Id, Id)>> {
        let q = format!(
            r#"match
                $u isa user, has user-sub "{}";
                membership (member: $u, org: $o);
                $o has org-id $oid;
                allocation (org: $o, hub: $h);
                $h has hub-id $hid;
            fetch {{ "org": $oid, "hub": $hid }};"#,
            Self::esc(subject)
        );
        let rows = self.fetch_all(&q).await?;
        Ok(rows.first().and_then(|d| {
            let org = d.get("org").and_then(|v| v.as_str()).map(str::to_string)?;
            let hub = d.get("hub").and_then(|v| v.as_str()).map(str::to_string)?;
            Some((org, hub))
        }))
    }

    /// Users belonging to an organization.
    pub async fn users_for_org(&self, org_id: &str) -> anyhow::Result<Vec<Value>> {
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                membership (member: $u, org: $o);
            fetch {{ "user": {{ $u.* }} }};"#,
            org = Self::esc(org_id)
        );
        self.fetch_all(&q).await
    }

    /// Hubs allocated to an organization.
    pub async fn hubs_for_org(&self, org_id: &str) -> anyhow::Result<Vec<Value>> {
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                allocation (org: $o, hub: $h);
            fetch {{ "hub": {{ $h.* }} }};"#,
            org = Self::esc(org_id)
        );
        self.fetch_all(&q).await
    }

    /// Users with a `driver` role assignment within an organization.
    pub async fn drivers_for_org(&self, org_id: &str) -> anyhow::Result<Vec<Value>> {
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                membership (member: $u, org: $o);
                $r isa role, has role-name "driver";
                assignment (member: $u, role: $r);
            fetch {{ "driver": {{ $u.* }} }};"#,
            org = Self::esc(org_id)
        );
        self.fetch_all(&q).await
    }

    /// Insert a driver expense entry.
    pub async fn record_cost(&self, entry: &altius_core::CostEntry, driver_sub: &str) -> anyhow::Result<()> {
        let tx = self
            .driver
            .transaction(&self.database, TransactionType::Write)
            .await
            .context("open write tx for cost")?;
        let q = format!(
            r#"insert
                $e isa cost-entry,
                    has event-id "{id}",
                    has category "{category}",
                    has amount-minor {amount},
                    has currency "{currency}",
                    has note "{note}",
                    has day "{day}",
                    has user-sub "{driver}";"#,
            id = Self::esc(&entry.id),
            category = Self::esc(&format!("{:?}", entry.category).to_lowercase()),
            amount = entry.amount_minor,
            currency = Self::esc(&entry.currency),
            note = Self::esc(&entry.note),
            day = Self::esc(&entry.day),
            driver = Self::esc(driver_sub),
        );
        tx.query(&q).await.context("insert cost")?;
        tx.commit().await.context("commit cost")?;
        Ok(())
    }

    /// List cost entries for a driver, optionally filtered to a day.
    pub async fn costs_for_driver(&self, driver_sub: &str, day: Option<&str>) -> anyhow::Result<Vec<Value>> {
        let day_filter = day.map_or(String::new(), |d| format!(r#", has day "{}""#, Self::esc(d)));
        let q = format!(
            r#"match
                $e isa cost-entry, has user-sub "{driver}" {day_filter};
            fetch {{ "entry": {{ $e.* }} }};"#,
            driver = Self::esc(driver_sub),
        );
        self.fetch_all(&q).await
    }

    /// Insert a daily LHS report.
    pub async fn record_daily_report(&self, report: &altius_core::DailyReport, driver_sub: &str) -> anyhow::Result<()> {
        let tx = self
            .driver
            .transaction(&self.database, TransactionType::Write)
            .await
            .context("open write tx for report")?;
        let payload = serde_json::to_string(report)?;
        let q = format!(
            r#"match
                $d isa user, has user-sub "{driver}";
            insert
                $r isa daily-report,
                    has day "{day}",
                    has report-status "{status}",
                    has revision {revision},
                    has payload "{payload}";
                submitted (driver: $d, report: $r);"#,
            driver = Self::esc(driver_sub),
            day = Self::esc(&report.day),
            status = Self::esc(&format!("{:?}", report.status).to_lowercase()),
            revision = report.revision,
            payload = Self::esc(&payload),
        );
        tx.query(&q).await.context("insert report")?;
        tx.commit().await.context("commit report")?;
        Ok(())
    }

    /// List daily reports submitted by a driver.
    pub async fn reports_for_driver(&self, driver_sub: &str) -> anyhow::Result<Vec<Value>> {
        let q = format!(
            r#"match
                $d isa user, has user-sub "{driver}";
                submitted (driver: $d, report: $r);
            fetch {{ "report": {{ $r.* }} }};"#,
            driver = Self::esc(driver_sub)
        );
        self.fetch_all(&q).await
    }
}
