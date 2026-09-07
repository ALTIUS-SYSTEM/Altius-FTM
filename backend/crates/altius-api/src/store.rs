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

    fn esc(s: &str) -> String {
        s.replace('\\', "\\\\").replace('"', "\\\"")
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
    pub async fn create_task(&self, task: &Task) -> anyhow::Result<()> {
        let tx = self
            .driver
            .transaction(&self.database, TransactionType::Write)
            .await
            .context("open write tx")?;

        let stage = serde_json::to_value(task.status)?
            .as_str()
            .unwrap_or("assigned")
            .to_string();
        let mut q = format!(
            r#"match
                $o isa organization, has org-id "{org}";
                $h isa hub, has hub-id "{hub}";
            insert
                $t isa task,
                    has task-id "{tid}",
                    has title "{title}",
                    has stage "{stage}",
                    has day "{day}";
                located (task: $t, hub: $h);"#,
            org = Self::esc(&task.tenant_id),
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
    pub async fn record_event(&self, ev: &DeviceEvent) -> anyhow::Result<EventReceipt> {
        let tx = self
            .driver
            .transaction(&self.database, TransactionType::Write)
            .await
            .context("open write tx")?;

        let check = format!(
            r#"match $e isa device-event, has request-key "{}"; select $e;"#,
            Self::esc(&ev.idempotency_key)
        );
        if let QueryAnswer::ConceptRowStream(_, mut rows) =
            tx.query(&check).await.context("idempotency check")?
            && let Some(row) = rows.next().await
        {
            row?;
            tx.rollback().await.ok();
            return Ok(EventReceipt::Accepted {
                server_event_id: ev.idempotency_key.clone(),
            });
        }

        // Read current stop stage to enforce the transition rules.
        let mut new_stage: Option<StopStatus> = None;
        if let Some(stop_id) = &ev.stop_id {
            let stage_q = format!(
                r#"match $s isa stop, has stop-id "{}", has stage $st; fetch {{ "stage": $st }};"#,
                Self::esc(stop_id)
            );
            if let QueryAnswer::ConceptDocumentStream(_, mut docs) =
                tx.query(&stage_q).await.context("read stop stage")?
                && let Some(doc) = docs.next().await {
                    let v = serde_json::to_value(doc?.into_json())?;
                    if let Some(cur) = v.get("stage").and_then(|s| s.as_str()) {
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
                $t isa task, has task-id "{tid}";
            insert
                recorded (event: $e, task: $t);"#,
            id = Self::esc(&ev.event_id),
            key = Self::esc(&ev.idempotency_key),
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
                    $s isa stop, has stop-id "{sid}", has stage $old;
                delete has $old of $s;
                insert
                    $s has stage "{stage}";
                    reports (stop: $s, event: $e);"#,
                eid = Self::esc(&ev.event_id),
                sid = Self::esc(stop_id),
                stage = Self::esc(&stage_name),
            );
            tx.query(&link).await.context("link stop + advance")?;
        }

        // Roll the task stage forward when a stop departs.
        if matches!(new_stage, Some(StopStatus::Departed)) {
            let roll = format!(
                r#"match
                    $t isa task, has task-id "{tid}", has stage $old;
                delete has $old of $t;
                insert $t has stage "in_progress";"#,
                tid = Self::esc(&ev.task_id),
            );
            tx.query(&roll).await.context("advance task stage")?;
        }

        tx.commit().await.context("commit")?;
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
}
