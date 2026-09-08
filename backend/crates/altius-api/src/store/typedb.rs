//! TypeDB persistence — atomic event+task writes mirroring the mobile
//! `WorkStore` guarantees (single transaction, immutable events, request-key
//! idempotency).

use std::sync::Arc;

use anyhow::Context;
use futures::StreamExt;
use serde_json::Value;
use typedb_driver::answer::QueryAnswer;
use typedb_driver::{TransactionType, TypeDBDriver};

use altius_core::{DeviceEvent, EventReceipt, GpsQuality, Id, StopStatus, Task, check_transition};

pub struct TypedbStore {
    driver: Arc<TypeDBDriver>,
    database: String,
}

impl Clone for TypedbStore {
    fn clone(&self) -> Self {
        Self {
            driver: Arc::clone(&self.driver),
            database: self.database.clone(),
        }
    }
}

impl TypedbStore {
    pub fn new(driver: TypeDBDriver, database: String) -> Self {
        Self {
            driver: Arc::new(driver),
            database,
        }
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

    /// Tasks assigned to a specific driver — the `dispatch` relation ties
    /// task → user. Matches the driver's scoped board on the Postgres path.
    pub async fn tasks_for_driver(
        &self,
        org_id: &str,
        driver_sub: &str,
    ) -> anyhow::Result<Vec<Value>> {
        let q = format!(
            r#"match
                allocation (org: $o, hub: $h);
                $o has org-id "{org}";
                located (task: $t, hub: $h);
                dispatch (task: $t, driver: $d);
                $d has user-sub "{driver}";
            fetch {{
                "task": {{ $t.* }},
                "stops": [
                    match contains (parent: $t, child: $s);
                    fetch {{ "stop": {{ $s.* }} }};
                ]
            }};"#,
            org = Self::esc(org_id),
            driver = Self::esc(driver_sub)
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
        // Every clause above already ends in ';' — the base insert and each
        // stop block — so appending another produced an empty statement.
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
                event_id: ev.event_id.clone(),
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
                && let Some(doc) = docs.next().await
            {
                let v = serde_json::to_value(doc?.into_json())?;
                if let Some(cur) = v.get("stage").and_then(|s| s.as_str()) {
                    resolved = true;
                    let Ok(from) = serde_json::from_str::<StopStatus>(&format!("\"{cur}\"")) else {
                        tx.rollback().await.ok();
                        return Ok(EventReceipt::Conflict {
                            event_id: ev.event_id.clone(),
                            reason: format!("stop {stop_id} has unknown stage {cur:?}"),
                        });
                    };
                    match check_transition(from, ev.action) {
                        Ok(next) => new_stage = Some(next),
                        Err(e) => {
                            tx.rollback().await.ok();
                            return Ok(EventReceipt::Conflict {
                                event_id: ev.event_id.clone(),
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
                    event_id: ev.event_id.clone(),
                    reason: format!(
                        "stop {stop_id} is not part of task {} in this organization",
                        ev.task_id
                    ),
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

        // Roll the task stage forward on arrive and when every stop is terminal.
        // Mobile ends at complete_activity; do not wait for a separate depart.
        if matches!(
            new_stage,
            Some(StopStatus::Arrived | StopStatus::Working | StopStatus::Departed)
        ) {
            let roll = format!(
                r#"match
                    $o isa organization, has org-id "{org}";
                    allocation (org: $o, hub: $h);
                    located (task: $t, hub: $h);
                    $t has task-id "{tid}", has stage $old;
                    $old == "assigned";
                delete has $old of $t;
                insert $t has stage "in_progress";"#,
                org = Self::esc(org),
                tid = Self::esc(&ev.task_id),
            );
            tx.query(&roll).await.context("advance task stage")?;
        }
        if matches!(
            new_stage,
            Some(StopStatus::Completed | StopStatus::Departed | StopStatus::Skipped)
        ) {
            let done = format!(
                r#"match
                    $o isa organization, has org-id "{org}";
                    allocation (org: $o, hub: $h);
                    located (task: $t, hub: $h);
                    $t has task-id "{tid}", has stage $old;
                    not {{
                        contains (parent: $t, child: $s);
                        $s isa stop, has stage $st;
                        {{ $st == "pending"; }} or {{ $st == "arrived"; }} or {{ $st == "working"; }}
                    }};
                delete has $old of $t;
                insert $t has stage "completed";"#,
                org = Self::esc(org),
                tid = Self::esc(&ev.task_id),
            );
            // Best-effort: if a stop is still open the match fails and we leave
            // the task stage alone.
            let _ = tx.query(&done).await;
        }

        // If the event carries an app GPS fix, mirror it as an observation for
        // comparison with the vehicle telematics stream (McEasy VSMS). The
        // observation id is derived from the event id, so a replay is naturally
        // idempotent and the uniqueness guard above protects it.
        if let Some(loc) = ev.location {
            let mut owns = Vec::new();
            let id = format!("obs:{}", ev.event_id);
            owns.push(format!(r#"has observation-id "{}""#, Self::esc(&id)));
            owns.push(format!(r#"has org-id "{}""#, Self::esc(&ev.tenant_id)));
            owns.push(format!(r#"has hub-id "{}""#, Self::esc(&ev.hub_id)));
            owns.push(format!(r#"has user-sub "{}""#, Self::esc(driver_sub)));
            owns.push(r#"has source "app_gps""#.to_string());
            let quality = match ev.accuracy_meters {
                Some(a) if a <= 50.0 => GpsQuality::Accurate,
                Some(_) => GpsQuality::Degraded,
                None => GpsQuality::Accurate,
            };
            owns.push(format!(
                r#"has quality "{}""#,
                Self::esc(&format!("{:?}", quality).to_lowercase())
            ));
            owns.push(format!("has latitude {}", loc.lat));
            owns.push(format!("has longitude {}", loc.lng));
            if let Some(a) = ev.accuracy_meters {
                owns.push(format!("has accuracy-meters {a}"));
            }
            owns.push(format!(
                r#"has recorded-at-utc {}"#,
                ev.time.utc.format("%Y-%m-%dT%H:%M:%S%.3f+00:00")
            ));
            owns.push(format!(
                r#"has day "{}""#,
                Self::esc(&ev.time.utc.date_naive().to_string())
            ));

            let q = format!(
                r#"match
                    $o isa organization, has org-id "{org}";
                    $h isa hub, has hub-id "{hub}";
                    allocation (org: $o, hub: $h);
                insert
                    $obs isa gps-observation, {owns};"#,
                org = Self::esc(&ev.tenant_id),
                hub = Self::esc(&ev.hub_id),
                owns = owns.join(", "),
            );
            tx.query(&q).await.context("insert app gps observation")?;
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
                    event_id: ev.event_id.clone(),
                    server_event_id: ev.event_id.clone(),
                });
            }
            return Err(anyhow::Error::new(e).context("commit"));
        }
        Ok(EventReceipt::Accepted {
            event_id: ev.event_id.clone(),
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
    pub async fn ensure_default_org_hub(
        &self,
        config: &crate::config::Config,
    ) -> anyhow::Result<()> {
        let tx = self
            .driver
            .transaction(&self.database, TransactionType::Write)
            .await
            .context("open write tx for provisioning")?;
        // Guard the insert. `org-id` is `@key`, and re-inserting an existing key
        // raises a constraint violation rather than being ignored — this runs on
        // every boot, so an unguarded insert let the service start exactly once
        // and fail every restart afterwards.
        let already = format!(
            r#"match $o isa organization, has org-id "{org}"; fetch {{ "found": true }};"#,
            org = Self::esc(&config.default_org_id),
        );
        if self.exists(&already).await? {
            tx.rollback().await.ok();
            return Ok(());
        }
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
        // Same guard as the org/hub bootstrap: `user-sub` is `@key`, and this
        // runs on every boot.
        let already = format!(
            r#"match $u isa user, has user-sub "{sub}"; fetch {{ "found": true }};"#,
            sub = Self::esc(&config.default_admin_sub),
        );
        if self.exists(&already).await? {
            tx.rollback().await.ok();
            return Ok(());
        }
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
            insert
                $u isa user, has user-sub "{sub}", has role-name "admin";
                membership (member: $u, org: $o);"#,
            org = Self::esc(&config.default_org_id),
            sub = Self::esc(&config.default_admin_sub),
        );
        tx.query(&q).await.context("link admin user")?;
        tx.commit().await.context("commit admin link")?;
        Ok(())
    }

    /// Bind a freshly created Keycloak subject to an org and hub.
    ///
    /// `org` and `hub` come from the calling admin's own resolved scope, never
    /// from the request body — otherwise provisioning becomes a way to plant a
    /// member in another tenant. The hub must be allocated to that org.
    pub async fn provision_user(
        &self,
        org: &str,
        hub: &str,
        subject: &str,
        display_name: &str,
        role: &str,
    ) -> anyhow::Result<()> {
        let tx = self
            .driver
            .transaction(&self.database, TransactionType::Write)
            .await
            .context("open write tx for provisioning")?;
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                $h isa hub, has hub-id "{hub}";
                allocation (org: $o, hub: $h);
            insert
                $u isa user, has user-sub "{sub}", has display-name "{name}",
                    has role-name "{role}";
                membership (member: $u, org: $o);
                hub-assignment (member: $u, hub: $h);"#,
            org = Self::esc(org),
            hub = Self::esc(hub),
            sub = Self::esc(subject),
            name = Self::esc(display_name),
            role = Self::esc(role),
        );
        tx.query(&q).await.context("provision user")?;
        tx.commit().await.context("commit provisioning")?;
        Ok(())
    }

    /// Run a write query and report whether it matched anything.
    ///
    /// TypeQL makes an `insert` whose `match` is empty a silent no-op, so every
    /// scoped mutation must check rather than assume — otherwise a caller
    /// targeting another tenant's row gets HTTP 200 and no change.
    async fn write_scoped(&self, query: &str, context_msg: &'static str) -> anyhow::Result<bool> {
        let tx = self
            .driver
            .transaction(&self.database, TransactionType::Write)
            .await
            .context("open write tx")?;
        let answer = tx.query(query).await.context(context_msg)?;
        let touched = match answer {
            QueryAnswer::ConceptRowStream(_, mut rows) => rows.next().await.is_some(),
            QueryAnswer::ConceptDocumentStream(_, mut docs) => docs.next().await.is_some(),
            QueryAnswer::Ok(_) => true,
        };
        if touched {
            tx.commit().await.context("commit")?;
        } else {
            tx.rollback().await.ok();
        }
        Ok(touched)
    }

    async fn exists(&self, query: &str) -> anyhow::Result<bool> {
        Ok(!self.fetch_all(query).await?.is_empty())
    }

    // ---------- hubs ----------

    pub async fn create_hub(
        &self,
        org: &str,
        hub_id: &str,
        name: &str,
        lat: f64,
        lng: f64,
    ) -> anyhow::Result<bool> {
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
            insert
                $h isa hub, has hub-id "{hub}", has display-name "{name}",
                    has latitude {lat}, has longitude {lng};
                allocation (org: $o, hub: $h);"#,
            org = Self::esc(org),
            hub = Self::esc(hub_id),
            name = Self::esc(name),
        );
        self.write_scoped(&q, "create hub").await
    }

    pub async fn update_hub(
        &self,
        org: &str,
        hub_id: &str,
        name: &str,
        lat: f64,
        lng: f64,
    ) -> anyhow::Result<bool> {
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                $h isa hub, has hub-id "{hub}";
                allocation (org: $o, hub: $h);
                $h has display-name $old-name, has latitude $old-lat, has longitude $old-lng;
            delete
                has $old-name of $h;
                has $old-lat of $h;
                has $old-lng of $h;
            insert
                $h has display-name "{name}", has latitude {lat}, has longitude {lng};"#,
            org = Self::esc(org),
            hub = Self::esc(hub_id),
            name = Self::esc(name),
        );
        self.write_scoped(&q, "update hub").await
    }

    /// Whether anything still references this hub. Deleting a hub with tasks or
    /// teams would orphan them, so the route refuses instead of cascading.
    pub async fn hub_in_use(&self, org: &str, hub_id: &str) -> anyhow::Result<bool> {
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                $h isa hub, has hub-id "{hub}";
                allocation (org: $o, hub: $h);
                {{ located (task: $x, hub: $h); }}
                    or {{ stationed (team: $x, hub: $h); }}
                    or {{ hub-assignment (member: $x, hub: $h); }}
                    or {{ operates (hub: $h, vehicle: $x); }};
            fetch {{ "used": true }};"#,
            org = Self::esc(org),
            hub = Self::esc(hub_id),
        );
        self.exists(&q).await
    }

    pub async fn delete_hub(&self, org: &str, hub_id: &str) -> anyhow::Result<bool> {
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                $h isa hub, has hub-id "{hub}";
                $a isa allocation (org: $o, hub: $h);
            delete
                $a;
                $h;"#,
            org = Self::esc(org),
            hub = Self::esc(hub_id),
        );
        self.write_scoped(&q, "delete hub").await
    }

    // ---------- organization ----------

    /// Rename only. Creating and deleting organizations is tenant lifecycle,
    /// not an in-app admin form — a delete would cascade every hub, task and
    /// report the tenant owns.
    pub async fn update_organization(&self, org: &str, name: &str) -> anyhow::Result<bool> {
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}", has display-name $old;
            delete has $old of $o;
            insert $o has display-name "{name}";"#,
            org = Self::esc(org),
            name = Self::esc(name),
        );
        self.write_scoped(&q, "update organization").await
    }

    // ---------- teams ----------

    pub async fn teams_for_org(&self, org: &str) -> anyhow::Result<Vec<Value>> {
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                allocation (org: $o, hub: $h);
                stationed (team: $t, hub: $h);
                $h has hub-id $hid;
            fetch {{
                "team": {{ $t.* }},
                "hub": $hid,
                "members": [
                    match crewing (team: $t, member: $u);
                    fetch {{ "user": {{ $u.* }} }};
                ]
            }};"#,
            org = Self::esc(org)
        );
        self.fetch_all(&q).await
    }

    pub async fn create_team(
        &self,
        org: &str,
        hub_id: &str,
        team_id: &str,
        name: &str,
        shift: &str,
    ) -> anyhow::Result<bool> {
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                $h isa hub, has hub-id "{hub}";
                allocation (org: $o, hub: $h);
            insert
                $t isa team, has team-id "{tid}", has display-name "{name}", has shift "{shift}";
                stationed (team: $t, hub: $h);"#,
            org = Self::esc(org),
            hub = Self::esc(hub_id),
            tid = Self::esc(team_id),
            name = Self::esc(name),
            shift = Self::esc(shift),
        );
        self.write_scoped(&q, "create team").await
    }

    pub async fn update_team(
        &self,
        org: &str,
        team_id: &str,
        name: &str,
        shift: &str,
    ) -> anyhow::Result<bool> {
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                allocation (org: $o, hub: $h);
                stationed (team: $t, hub: $h);
                $t has team-id "{tid}", has display-name $old-name, has shift $old-shift;
            delete
                has $old-name of $t;
                has $old-shift of $t;
            insert
                $t has display-name "{name}", has shift "{shift}";"#,
            org = Self::esc(org),
            tid = Self::esc(team_id),
            name = Self::esc(name),
            shift = Self::esc(shift),
        );
        self.write_scoped(&q, "update team").await
    }

    pub async fn delete_team(&self, org: &str, team_id: &str) -> anyhow::Result<bool> {
        let crew = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                allocation (org: $o, hub: $h);
                stationed (team: $t, hub: $h);
                $t has team-id "{tid}";
                $c isa crewing (team: $t, member: $u);
            delete $c;"#,
            org = Self::esc(org),
            tid = Self::esc(team_id),
        );
        // Roster links first: TypeDB refuses to delete an entity that still
        // plays a role in a relation, and an empty roster is not an error.
        self.write_scoped(&crew, "clear team roster").await.ok();

        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                allocation (org: $o, hub: $h);
                $st isa stationed (team: $t, hub: $h);
                $t has team-id "{tid}";
            delete
                $st;
                $t;"#,
            org = Self::esc(org),
            tid = Self::esc(team_id),
        );
        self.write_scoped(&q, "delete team").await
    }

    /// Add a driver to a team. Both must already sit inside the caller's org,
    /// so a subject from another tenant simply fails to match.
    pub async fn add_team_member(
        &self,
        org: &str,
        team_id: &str,
        subject: &str,
    ) -> anyhow::Result<bool> {
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                allocation (org: $o, hub: $h);
                stationed (team: $t, hub: $h);
                $t has team-id "{tid}";
                $u isa user, has user-sub "{sub}";
                membership (member: $u, org: $o);
            insert
                crewing (team: $t, member: $u);"#,
            org = Self::esc(org),
            tid = Self::esc(team_id),
            sub = Self::esc(subject),
        );
        self.write_scoped(&q, "add team member").await
    }

    pub async fn remove_team_member(
        &self,
        org: &str,
        team_id: &str,
        subject: &str,
    ) -> anyhow::Result<bool> {
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                allocation (org: $o, hub: $h);
                stationed (team: $t, hub: $h);
                $t has team-id "{tid}";
                $u isa user, has user-sub "{sub}";
                $c isa crewing (team: $t, member: $u);
            delete $c;"#,
            org = Self::esc(org),
            tid = Self::esc(team_id),
            sub = Self::esc(subject),
        );
        self.write_scoped(&q, "remove team member").await
    }

    /// Refresh the cached realm role after Keycloak has been updated.
    ///
    /// Best-effort by design: Keycloak is the authority, so a stale cache here
    /// affects only which names appear in a roster, never what anyone may do.
    pub async fn set_cached_role(
        &self,
        org: &str,
        subject: &str,
        role: &str,
    ) -> anyhow::Result<bool> {
        let clear = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                $u isa user, has user-sub "{sub}", has role-name $old;
                membership (member: $u, org: $o);
            delete has $old of $u;"#,
            org = Self::esc(org),
            sub = Self::esc(subject),
        );
        self.write_scoped(&clear, "clear cached role").await.ok();
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                $u isa user, has user-sub "{sub}";
                membership (member: $u, org: $o);
            insert $u has role-name "{role}";"#,
            org = Self::esc(org),
            sub = Self::esc(subject),
            role = Self::esc(role),
        );
        self.write_scoped(&q, "set cached role").await
    }

    /// Confirm a subject belongs to the caller's organization before any
    /// operation that names them (role changes, team membership).
    pub async fn user_in_org(&self, org: &str, subject: &str) -> anyhow::Result<bool> {
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                $u isa user, has user-sub "{sub}";
                membership (member: $u, org: $o);
            fetch {{ "member": true }};"#,
            org = Self::esc(org),
            sub = Self::esc(subject),
        );
        self.exists(&q).await
    }

    /// Resolve the organization and first hub bound to a principal subject.
    pub async fn organization_and_hub_of(&self, subject: &str) -> anyhow::Result<Option<(Id, Id)>> {
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

    /// Upsert a push token for a user's device. Existing token for the same
    /// device id, owned by this same user, is replaced; a user can have
    /// multiple devices. A device id already owned by a *different* user is
    /// rejected rather than silently rebound: the previous fallback matched
    /// only `device-id`, so it overwrote another user's `fcm-token` in place
    /// while `owns` still pointed at the original owner -- that owner's
    /// pushes then silently went to whoever last wrote the token.
    /// Bind an FCM token to a device for this user.
    ///
    /// Read first, then write. The previous shape attempted the insert and
    /// recovered from the error, but a failed statement poisons the enclosing
    /// TypeDB transaction — every follow-up in it failed too, so a device could
    /// never be re-registered with a refreshed token.
    pub async fn register_push_token(
        &self,
        subject: &str,
        device_id: &str,
        token: &str,
    ) -> anyhow::Result<()> {
        let owner = format!(
            r#"match
                $d isa device, has device-id "{did}";
                registered (owner: $u, asset: $d);
                $u has user-sub $sub;
            fetch {{ "sub": $sub }};"#,
            did = Self::esc(device_id),
        );
        let existing: Option<String> = self
            .fetch_all(&owner)
            .await?
            .first()
            .and_then(|d| d.get("sub"))
            .and_then(|v| v.as_str())
            .map(str::to_string);

        match existing {
            // A device id already bound elsewhere is a conflict, not a rebind:
            // silently moving it would send one user's notifications to another.
            Some(other) if other != subject => {
                anyhow::bail!("device {device_id} is already registered to a different user")
            }
            Some(_) => {
                let q = format!(
                    r#"match
                        $d isa device, has device-id "{did}", has fcm-token $old;
                    delete has $old of $d;
                    insert $d has fcm-token "{token}";"#,
                    did = Self::esc(device_id),
                    token = Self::esc(token),
                );
                self.write_scoped(&q, "refresh push token").await?;
            }
            None => {
                let q = format!(
                    r#"match
                        $u isa user, has user-sub "{sub}";
                    insert
                        $d isa device, has device-id "{did}", has fcm-token "{token}";
                        registered (owner: $u, asset: $d);"#,
                    sub = Self::esc(subject),
                    did = Self::esc(device_id),
                    token = Self::esc(token),
                );
                if !self.write_scoped(&q, "register push token").await? {
                    anyhow::bail!("unknown user {subject}");
                }
            }
        }
        Ok(())
    }

    pub async fn push_tokens_for_user(&self, subject: &str) -> anyhow::Result<Vec<String>> {
        let q = format!(
            r#"match
                $u isa user, has user-sub "{sub}";
                registered (owner: $u, asset: $d);
                $d has fcm-token $t;
            fetch {{ "t": $t }};"#,
            sub = Self::esc(subject)
        );
        let rows = self.fetch_all(&q).await?;
        Ok(rows
            .iter()
            .filter_map(|d| d.get("t").and_then(|v| v.as_str()).map(str::to_string))
            .collect())
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
                $u has role-name "driver";
            fetch {{ "driver": {{ $u.* }} }};"#,
            org = Self::esc(org_id)
        );
        self.fetch_all(&q).await
    }

    /// Insert a driver expense entry.
    pub async fn record_cost(
        &self,
        _org: &str,
        _hub: &str,
        entry: &altius_core::CostEntry,
        driver_sub: &str,
    ) -> anyhow::Result<()> {
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
    pub async fn costs_for_driver(
        &self,
        _org: &str,
        driver_sub: &str,
        day: Option<&str>,
    ) -> anyhow::Result<Vec<Value>> {
        let day_filter = day.map_or(String::new(), |d| {
            format!(r#", has day "{}""#, Self::esc(d))
        });
        let q = format!(
            r#"match
                $e isa cost-entry, has user-sub "{driver}" {day_filter};
            fetch {{ "entry": {{ $e.* }} }};"#,
            driver = Self::esc(driver_sub),
        );
        self.fetch_all(&q).await
    }

    /// Insert a daily LHS report.
    pub async fn record_daily_report(
        &self,
        _org: &str,
        report: &altius_core::DailyReport,
        driver_sub: &str,
    ) -> anyhow::Result<()> {
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
    pub async fn reports_for_driver(&self, _org: &str, driver_sub: &str) -> anyhow::Result<Vec<Value>> {
        let q = format!(
            r#"match
                $d isa user, has user-sub "{driver}";
                submitted (driver: $d, report: $r);
            fetch {{ "report": {{ $r.* }} }};"#,
            driver = Self::esc(driver_sub)
        );
        self.fetch_all(&q).await
    }

    /// Record a daily vehicle checklist.
    pub async fn record_vehicle_check(
        &self,
        org: &str,
        hub: &str,
        check: &altius_core::VehicleCheck,
    ) -> anyhow::Result<()> {
        let tx = self
            .driver
            .transaction(&self.database, TransactionType::Write)
            .await
            .context("open write tx for vehicle check")?;
        let payload = Self::esc(&serde_json::to_string(&check.items).context("serialize items")?);
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                $h isa hub, has hub-id "{hub}";
                allocation (org: $o, hub: $h);
                $d isa user, has user-sub "{driver}";
                membership (member: $d, org: $o);
            insert
                $c isa vehicle-check,
                    has vehicle-check-id "{id}",
                    has day "{day}",
                    has driver-name "{name}",
                    has license-plate "{plate}",
                    has vehicle-type "{vtype}",
                    has km-start {km_start},
                    has km-end {km_end},
                    has vehicle-condition "{condition}",
                    has check-notes "{notes}",
                    has service-date "{service}",
                    has kir-date "{kir}",
                    has stnk-date "{stnk}",
                    has user-sub "{driver}",
                    has payload "{payload}";
                inspected (check: $c, driver: $d);"#,
            org = Self::esc(org),
            hub = Self::esc(hub),
            driver = Self::esc(&check.driver_id),
            id = Self::esc(&check.id),
            day = Self::esc(&check.day),
            name = Self::esc(&check.driver_name),
            plate = Self::esc(&check.license_plate),
            vtype = Self::esc(&check.vehicle_type),
            km_start = check.km_start,
            km_end = check.km_end,
            condition = Self::esc(match check.condition {
                altius_core::VehicleCondition::Good => "good",
                altius_core::VehicleCondition::NotGood => "not_good",
            }),
            notes = Self::esc(&check.notes),
            service = Self::esc(check.service_date.as_deref().unwrap_or("")),
            kir = Self::esc(check.kir_date.as_deref().unwrap_or("")),
            stnk = Self::esc(check.stnk_date.as_deref().unwrap_or("")),
            payload = payload,
        );
        tx.query(&q).await.context("insert vehicle check")?;
        tx.commit().await.context("commit vehicle check")?;
        Ok(())
    }

    /// Vehicle checks performed by drivers in this organization.
    ///
    /// `vehicle-check` has no relation to hub or organization in the schema —
    /// only `inspected` to the driver — so scope is derived through that
    /// driver's membership. Matching `$c isa vehicle-check` unqualified, as
    /// this did, returned every check in the database to any caller whose own
    /// organization merely existed.
    pub async fn vehicle_checks_for_org(
        &self,
        org: &str,
        hub: Option<&str>,
    ) -> anyhow::Result<Vec<Value>> {
        let hub_filter = hub
            .map(|h| {
                format!(
                    r#"hub-assignment (member: $d, hub: $h); $h isa hub, has hub-id "{h}";"#,
                    h = Self::esc(h)
                )
            })
            .unwrap_or_default();
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                $d isa user;
                membership (member: $d, org: $o);
                inspected (check: $c, driver: $d);
                {hub_filter}
                $d has user-sub $dsub;
            fetch {{ "check": {{ $c.* }}, "driver": $dsub }};"#,
            org = Self::esc(org),
            hub_filter = hub_filter
        );
        self.fetch_all(&q).await
    }

    // ---------- GPS observations & McEasy monitoring ----------

    /// Persist a GPS observation (app or vehicle) for monitoring and review.
    pub async fn record_gps_observation(
        &self,
        obs: &altius_core::GpsObservation,
    ) -> anyhow::Result<()> {
        let mut owns = Vec::new();
        owns.push(format!(r#"has observation-id "{}""#, Self::esc(&obs.id)));
        owns.push(format!(r#"has org-id "{}""#, Self::esc(&obs.tenant_id)));
        owns.push(format!(r#"has hub-id "{}""#, Self::esc(&obs.hub_id)));
        owns.push(format!(r#"has user-sub "{}""#, Self::esc(&obs.driver_id)));
        if let Some(plate) = &obs.vehicle_id {
            owns.push(format!(r#"has plate "{}""#, Self::esc(plate)));
        }
        owns.push(format!(
            r#"has source "{}""#,
            Self::esc(&format!("{:?}", obs.source).to_lowercase())
        ));
        owns.push(format!(
            r#"has quality "{}""#,
            Self::esc(&format!("{:?}", obs.quality).to_lowercase())
        ));
        if let Some(loc) = obs.location {
            owns.push(format!("has latitude {lat}", lat = loc.lat));
            owns.push(format!("has longitude {lng}", lng = loc.lng));
        }
        if let Some(a) = obs.accuracy_meters {
            owns.push(format!("has accuracy-meters {a}"));
        }
        if let Some(s) = obs.speed_mps {
            owns.push(format!("has speed-mps {s}"));
        }
        if let Some(m) = obs.mock_location_reported {
            owns.push(format!("has mock-reported {m}"));
        }
        owns.push(format!(
            r#"has recorded-at-utc {utc}"#,
            utc = obs.time.utc.format("%Y-%m-%dT%H:%M:%S%.3f+00:00")
        ));
        owns.push(format!(r#"has day "{}""#, Self::esc(&obs.day())));

        let tx = self
            .driver
            .transaction(&self.database, TransactionType::Write)
            .await
            .context("open write tx for gps observation")?;
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                $h isa hub, has hub-id "{hub}";
                allocation (org: $o, hub: $h);
            insert
                $obs isa gps-observation, {owns};"#,
            org = Self::esc(&obs.tenant_id),
            hub = Self::esc(&obs.hub_id),
            owns = owns.join(", "),
        );
        tx.query(&q).await.context("insert gps observation")?;
        tx.commit().await.context("commit gps observation")?;
        Ok(())
    }

    /// Persist the comparison result between an app and vehicle observation.
    pub async fn record_gps_review(&self, review: &altius_core::GpsReview) -> anyhow::Result<()> {
        let mut owns = Vec::new();
        owns.push(format!(r#"has review-id "{}""#, Self::esc(&review.id)));
        owns.push(format!(r#"has org-id "{}""#, Self::esc(&review.tenant_id)));
        owns.push(format!(r#"has hub-id "{}""#, Self::esc(&review.hub_id)));
        owns.push(format!(
            r#"has user-sub "{}""#,
            Self::esc(&review.driver_id)
        ));
        if let Some(plate) = &review.vehicle_id {
            owns.push(format!(r#"has plate "{}""#, Self::esc(plate)));
        }
        owns.push(format!(
            r#"has app-observation-id "{}""#,
            Self::esc(&review.app_observation_id)
        ));
        if let Some(vid) = &review.vehicle_observation_id {
            owns.push(format!(
                r#"has vehicle-observation-id "{}""#,
                Self::esc(vid)
            ));
        }
        owns.push(format!(
            r#"has classification "{}""#,
            Self::esc(&format!("{:?}", review.classification).to_lowercase())
        ));
        owns.push(format!(
            r#"has review-reason "{}""#,
            Self::esc(&format!("{:?}", review.reason).to_lowercase())
        ));
        if let Some(s) = review.separation_meters {
            owns.push(format!("has separation-meters {s}"));
        }
        if let Some(t) = review.time_delta_seconds {
            owns.push(format!("has time-delta-seconds {t}"));
        }
        if let Some(by) = &review.reviewed_by {
            owns.push(format!(r#"has reviewed-by "{}""#, Self::esc(by)));
        }
        owns.push(format!(
            r#"has recorded-at-utc {utc}"#,
            utc = review.created_at.format("%Y-%m-%dT%H:%M:%S%.3f+00:00")
        ));
        owns.push(format!(
            r#"has day "{}""#,
            Self::esc(&review.created_at.date_naive().to_string())
        ));

        let tx = self
            .driver
            .transaction(&self.database, TransactionType::Write)
            .await
            .context("open write tx for gps review")?;
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                $h isa hub, has hub-id "{hub}";
                allocation (org: $o, hub: $h);
            insert
                $r isa gps-review, {owns};"#,
            org = Self::esc(&review.tenant_id),
            hub = Self::esc(&review.hub_id),
            owns = owns.join(", "),
        );
        tx.query(&q).await.context("insert gps review")?;
        tx.commit().await.context("commit gps review")?;
        Ok(())
    }

    /// Mark a GPS review as reviewed by a super-admin.
    pub async fn mark_gps_review_reviewed(
        &self,
        org: &str,
        review_id: &str,
        reviewer: &str,
    ) -> anyhow::Result<bool> {
        let tx = self
            .driver
            .transaction(&self.database, TransactionType::Write)
            .await
            .context("open write tx for gps review review")?;
        let clear = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                $r isa gps-review, has review-id "{rid}", has reviewed-by $rb;
            delete has $rb of $r;"#,
            org = Self::esc(org),
            rid = Self::esc(review_id),
        );
        // Clearing is idempotent: if there is no existing attribute the match
        // is empty and the delete is a no-op.
        tx.query(&clear).await.ok();
        let insert = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                $r isa gps-review, has review-id "{rid}";
            insert
                $r has reviewed-by "{reviewer}";"#,
            org = Self::esc(org),
            rid = Self::esc(review_id),
            reviewer = Self::esc(reviewer),
        );
        tx.query(&insert).await.context("update gps review")?;
        tx.commit().await.context("commit gps review review")?;
        Ok(true)
    }

    /// GPS observations for an organization, optionally filtered by source and time.
    pub async fn gps_observations_for_org(
        &self,
        org: &str,
        source: Option<&str>,
        since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> anyhow::Result<Vec<Value>> {
        let source_filter = source
            .map(|s| format!(r#"$obs has source "{s}";"#, s = Self::esc(s)))
            .unwrap_or_default();
        let time_filter = since
            .map(|t| {
                format!(
                    r#"$obs has recorded-at-utc $t; $t >= {utc};"#,
                    utc = t.format("%Y-%m-%dT%H:%M:%S%.3f+00:00")
                )
            })
            .unwrap_or_default();
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                $obs isa gps-observation, has org-id "{org}";
                {source_filter}
                {time_filter}
            fetch {{ "observation": {{ $obs.* }} }};"#,
            org = Self::esc(org),
            source_filter = source_filter,
            time_filter = time_filter,
        );
        self.fetch_all(&q).await
    }

    /// GPS reviews for an organization.
    pub async fn gps_reviews_for_org(&self, org: &str) -> anyhow::Result<Vec<Value>> {
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                $r isa gps-review, has org-id "{org}";
            fetch {{ "review": {{ $r.* }} }};"#,
            org = Self::esc(org),
        );
        self.fetch_all(&q).await
    }

    /// Vehicles allocated to an organization that are candidates for monitoring.
    pub async fn monitoring_vehicles(&self, org: &str) -> anyhow::Result<Vec<Value>> {
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                $h isa hub;
                allocation (org: $o, hub: $h);
                $v isa vehicle;
                operates (hub: $h, vehicle: $v);
            fetch {{ "vehicle": {{ $v.* }}, "hub": {{ $h.* }} }};"#,
            org = Self::esc(org),
        );
        self.fetch_all(&q).await
    }

    /// All organizations in the database.
    pub async fn organizations(&self) -> anyhow::Result<Vec<String>> {
        let q = r#"match $o isa organization, has org-id $id; fetch { "id": $id };"#;
        Ok(self
            .fetch_all(q)
            .await?
            .iter()
            .filter_map(|v| v.get("id").and_then(|i| i.as_str()).map(str::to_string))
            .collect())
    }

    /// Update the McEasy vehicle id for a vehicle identified by license plate.
    pub async fn mceasy_sync_vehicle(
        &self,
        org: &str,
        plate: &str,
        mceasy_id: &str,
    ) -> anyhow::Result<()> {
        let tx = self
            .driver
            .transaction(&self.database, TransactionType::Write)
            .await
            .context("open write tx for mceasy vehicle sync")?;
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                $h isa hub;
                allocation (org: $o, hub: $h);
                $v isa vehicle, has plate "{plate}";
                operates (hub: $h, vehicle: $v);
            insert
                $v has mceasy-vehicle-id "{mceasy_id}";"#,
            org = Self::esc(org),
            plate = Self::esc(plate),
            mceasy_id = Self::esc(mceasy_id),
        );
        tx.query(&q).await.context("sync mceasy vehicle")?;
        tx.commit().await.context("commit mceasy vehicle sync")?;
        Ok(())
    }

    /// Update the McEasy driver id for a user in this organization.
    pub async fn mceasy_sync_driver(
        &self,
        org: &str,
        user_sub: &str,
        mceasy_id: &str,
    ) -> anyhow::Result<()> {
        let tx = self
            .driver
            .transaction(&self.database, TransactionType::Write)
            .await
            .context("open write tx for mceasy driver sync")?;
        let q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                $d isa user, has user-sub "{user_sub}";
                membership (member: $d, org: $o);
            insert
                $d has mceasy-driver-id "{mceasy_id}";"#,
            org = Self::esc(org),
            user_sub = Self::esc(user_sub),
            mceasy_id = Self::esc(mceasy_id),
        );
        tx.query(&q).await.context("sync mceasy driver")?;
        tx.commit().await.context("commit mceasy driver sync")?;
        Ok(())
    }

    /// Delete GPS observations older than the retention boundary.
    pub async fn prune_gps_observations_older_than(
        &self,
        boundary: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<()> {
        let q = format!(
            r#"match
                $obs isa gps-observation, has recorded-at-utc $t;
                $t < {utc};
            delete $obs;"#,
            utc = boundary.format("%Y-%m-%dT%H:%M:%S%.3f+00:00")
        );
        let tx = self
            .driver
            .transaction(&self.database, TransactionType::Write)
            .await
            .context("open write tx for gps prune")?;
        tx.query(&q).await.context("prune gps observations")?;
        tx.commit().await.context("commit gps prune")?;
        Ok(())
    }
}
