//! PostgreSQL persistence — the default transactional backend.
//!
//! Every query is scoped by `org_id` or `driver_sub` so a caller can never
//! reach another tenant's rows. `device_events` uses
//! `UNIQUE (org_id, driver_sub, request_key)` for idempotent event sync.

use anyhow::Context;
use deadpool_postgres::{Client, Pool};
use serde_json::Value;
use tokio_postgres::Row;
use tokio_postgres::types::ToSql;

use altius_core::{DeviceEvent, EventReceipt, GpsQuality, Id, StopStatus, Task, check_transition};

refinery::embed_migrations!("migrations");

/// Connect to PostgreSQL and build a connection pool.
///
/// `sslmode` in `DATABASE_URL` is not yet handled; production deployments are
/// expected to terminate TLS at the edge or run PostgreSQL on a private network.
pub async fn connect(url: &str) -> anyhow::Result<Pool> {
    let cfg: tokio_postgres::Config = url.parse().context("parse DATABASE_URL")?;
    let mgr = deadpool_postgres::Manager::new(cfg, tokio_postgres::NoTls);
    Ok(deadpool_postgres::Pool::builder(mgr).max_size(16).build()?)
}

/// Run embedded SQL migrations against the pool.
pub async fn migrate(pool: &Pool) -> anyhow::Result<()> {
    let mut client = pool.get().await.context("get migration connection")?;
    migrations::runner()
        .run_async(&mut **client)
        .await
        .context("run postgres migrations")?;
    Ok(())
}

#[derive(Clone)]
pub struct PgStore {
    pool: Pool,
}

impl PgStore {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Namespace a client idempotency key by tenant and driver so one caller
    /// cannot burn (and thereby silently suppress) another caller's key.
    fn request_key(org: &str, driver: &str, key: &str) -> String {
        format!("{org}\u{1f}{driver}\u{1f}{key}")
    }

    async fn conn(&self) -> anyhow::Result<Client> {
        self.pool.get().await.context("get postgres connection")
    }

    /// `row_to_json` / `jsonb_build_object` produce a single JSON column.
    fn json_row(row: &Row) -> Value {
        row.get(0)
    }

    async fn fetch_json(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> anyhow::Result<Vec<Value>> {
        let client = self.conn().await?;
        let rows = client.query(sql, params).await.context("postgres query")?;
        Ok(rows.iter().map(Self::json_row).collect())
    }

    async fn fetch_json_opt(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> anyhow::Result<Option<Value>> {
        let client = self.conn().await?;
        Ok(client
            .query_opt(sql, params)
            .await
            .context("postgres query")?
            .map(|r| Self::json_row(&r)))
    }

    async fn fetch_scalar<T>(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> anyhow::Result<Option<T>>
    where
        T: for<'a> tokio_postgres::types::FromSql<'a> + Send + Sync + 'static,
    {
        let client = self.conn().await?;
        Ok(client
            .query_opt(sql, params)
            .await
            .context("postgres query")?
            .map(|r| r.get(0)))
    }

    async fn fetch_bool(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> anyhow::Result<bool> {
        let client = self.conn().await?;
        let row = client
            .query_one(sql, params)
            .await
            .context("postgres query")?;
        Ok(row.get(0))
    }

    // ---------- tenant scoping ----------

    pub async fn organization_of(&self, subject: &str) -> anyhow::Result<Option<Id>> {
        self.fetch_scalar::<String>(
            "SELECT org_id FROM user_orgs WHERE user_sub = $1 LIMIT 1",
            &[&subject],
        )
        .await
    }

    pub async fn organization_and_hub_of(&self, subject: &str) -> anyhow::Result<Option<(Id, Id)>> {
        let client = self.conn().await?;
        let row = client
            .query_opt(
                "SELECT uo.org_id, uh.hub_id \
                 FROM user_orgs uo \
                 JOIN user_hubs uh ON uh.user_sub = uo.user_sub \
                 JOIN org_hubs oh ON oh.org_id = uo.org_id AND oh.hub_id = uh.hub_id \
                 WHERE uo.user_sub = $1 \
                 LIMIT 1",
                &[&subject],
            )
            .await
            .context("resolve org and hub")?;
        Ok(row.map(|r| (r.get::<usize, String>(0), r.get::<usize, String>(1))))
    }

    pub async fn user_in_org(&self, org: &str, subject: &str) -> anyhow::Result<bool> {
        self.fetch_bool(
            "SELECT EXISTS(SELECT 1 FROM user_orgs WHERE org_id = $1 AND user_sub = $2)",
            &[&org, &subject],
        )
        .await
    }

    // ---------- tasks & stops ----------

    pub async fn tasks_for_org(&self, org_id: &str) -> anyhow::Result<Vec<Value>> {
        self.fetch_json(
            "SELECT jsonb_build_object( \
                'task', row_to_json(t), \
                'stops', COALESCE(( \
                    SELECT jsonb_agg(row_to_json(s) ORDER BY s.sequence) \
                    FROM stops s WHERE s.task_id = t.id \
                ), '[]'::jsonb) \
             ) AS data \
             FROM tasks t \
             WHERE t.org_id = $1 \
             ORDER BY t.id",
            &[&org_id],
        )
        .await
    }

    pub async fn task_by_id(&self, org_id: &str, task_id: &str) -> anyhow::Result<Option<Value>> {
        self.fetch_json_opt(
            "SELECT jsonb_build_object( \
                'task', row_to_json(t), \
                'stops', COALESCE(( \
                    SELECT jsonb_agg(row_to_json(s) ORDER BY s.sequence) \
                    FROM stops s WHERE s.task_id = t.id \
                ), '[]'::jsonb) \
             ) AS data \
             FROM tasks t \
             WHERE t.org_id = $1 AND t.id = $2",
            &[&org_id, &task_id],
        )
        .await
    }

    pub async fn create_task(&self, org: &str, task: &Task) -> anyhow::Result<()> {
        let mut client = self.conn().await?;
        let tx = client.transaction().await.context("open create_task tx")?;

        let allocated: bool = tx
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM org_hubs WHERE org_id = $1 AND hub_id = $2)",
                &[&org, &task.hub_id],
            )
            .await
            .context("verify hub allocation")?
            .get(0);
        if !allocated {
            anyhow::bail!("hub {} is not allocated to organization {org}", task.hub_id);
        }

        let stage = serde_json::to_value(task.status)?
            .as_str()
            .unwrap_or("assigned")
            .to_string();
        tx.execute(
            "INSERT INTO tasks (id, org_id, hub_id, title, stage, day, created_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
            &[
                &task.id,
                &org,
                &task.hub_id,
                &task.title,
                &stage,
                &task.created_at.date_naive().to_string(),
                &task.created_at,
            ],
        )
        .await
        .context("insert task")?;

        for stop in &task.stops {
            let sstage = serde_json::to_value(stop.status)?
                .as_str()
                .unwrap_or("pending")
                .to_string();
            tx.execute(
                "INSERT INTO stops (id, task_id, sequence, name, address, lat, lng, stage, service_seconds) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
                &[
                    &stop.id,
                    &task.id,
                    &(stop.sequence as i64),
                    &stop.name,
                    &stop.address,
                    &stop.location.lat,
                    &stop.location.lng,
                    &sstage,
                    &(stop.service_seconds as i64),
                ],
            )
            .await
            .context("insert stop")?;
        }

        if let Some(assignee) = &task.assignee_id {
            tx.execute(
                "INSERT INTO task_assignments (task_id, driver_sub) VALUES ($1,$2) ON CONFLICT DO NOTHING",
                &[&task.id, assignee],
            )
            .await
            .context("assign task")?;
        }

        tx.commit().await.context("commit create_task")?;
        Ok(())
    }

    /// Append a device event atomically: insert the immutable event, update the
    /// stop stage, and roll the task stage forward — all in one transaction.
    /// A replayed request key short-circuits to `Accepted` without re-writing.
    pub async fn record_event(
        &self,
        org: &str,
        driver_sub: &str,
        ev: &DeviceEvent,
    ) -> anyhow::Result<EventReceipt> {
        let mut client = self.conn().await?;
        let tx = client.transaction().await.context("open record_event tx")?;

        let request_key = Self::request_key(org, driver_sub, &ev.idempotency_key);
        if let Some(existing) = tx
            .query_opt(
                "SELECT id FROM device_events \
                 WHERE org_id = $1 AND driver_sub = $2 AND request_key = $3",
                &[&org, &driver_sub, &request_key],
            )
            .await
            .context("idempotency check")?
        {
            let id: String = existing.get(0);
            return Ok(EventReceipt::Accepted {
                server_event_id: id,
            });
        }

        let mut new_stage: Option<StopStatus> = None;
        if let Some(stop_id) = &ev.stop_id {
            let row = tx
                .query_opt(
                    "SELECT s.stage \
                     FROM stops s \
                     JOIN tasks t ON t.id = s.task_id \
                     JOIN org_hubs oh ON oh.hub_id = t.hub_id \
                     WHERE oh.org_id = $1 AND t.id = $2 AND s.id = $3 \
                     FOR UPDATE OF s",
                    &[&org, &ev.task_id, &stop_id],
                )
                .await
                .context("read stop stage")?;
            let Some(row) = row else {
                return Ok(EventReceipt::Conflict {
                    reason: format!(
                        "stop {stop_id} is not part of task {} in this organization",
                        ev.task_id
                    ),
                });
            };
            let cur: String = row.get(0);
            let Ok(from) = serde_json::from_str::<StopStatus>(&format!("\"{cur}\"")) else {
                return Ok(EventReceipt::Conflict {
                    reason: format!("stop {stop_id} has unknown stage {cur:?}"),
                });
            };
            match check_transition(from, ev.action) {
                Ok(next) => new_stage = Some(next),
                Err(e) => {
                    return Ok(EventReceipt::Conflict {
                        reason: e.to_string(),
                    });
                }
            }
        }

        let action = serde_json::to_value(ev.action)?
            .as_str()
            .unwrap_or_default()
            .to_string();
        tx.execute(
            "INSERT INTO device_events \
             (id, org_id, driver_sub, device_id, task_id, stop_id, request_key, action, occurred_utc, offset_min, day, payload) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
            &[
                &ev.event_id,
                &org,
                &driver_sub,
                &ev.device_id,
                &ev.task_id,
                &ev.stop_id,
                &request_key,
                &action,
                &ev.time.utc,
                &ev.time.offset_minutes,
                &ev.time.utc.date_naive().to_string(),
                &ev.payload,
            ],
        )
        .await
        .context("insert device_event")?;

        if let (Some(stop_id), Some(stage)) = (&ev.stop_id, new_stage) {
            let stage_name = serde_json::to_value(stage)?
                .as_str()
                .unwrap_or_default()
                .to_string();
            tx.execute(
                "UPDATE stops SET stage = $4 \
                 WHERE id = $3 AND task_id = $2 AND task_id IN ( \
                     SELECT t.id FROM tasks t JOIN org_hubs oh ON oh.hub_id = t.hub_id \
                     WHERE oh.org_id = $1 \
                 )",
                &[&org, &ev.task_id, stop_id, &stage_name],
            )
            .await
            .context("advance stop stage")?;
        }

        if matches!(new_stage, Some(StopStatus::Departed)) {
            tx.execute(
                "UPDATE tasks SET stage = 'in_progress' WHERE id = $2 AND org_id = $1",
                &[&org, &ev.task_id],
            )
            .await
            .context("advance task stage")?;
        }

        if let Some(loc) = ev.location {
            let id = format!("obs:{}", ev.event_id);
            let quality = match ev.accuracy_meters {
                Some(a) if a <= 50.0 => GpsQuality::Accurate,
                Some(_) => GpsQuality::Degraded,
                None => GpsQuality::Accurate,
            };
            let quality_name = serde_json::to_value(quality)?
                .as_str()
                .unwrap_or("accurate")
                .to_string();
            tx.execute(
                "INSERT INTO gps_observations \
                 (id, org_id, hub_id, driver_sub, plate, source, quality, lat, lng, accuracy_meters, speed_mps, mock_reported, recorded_at, day) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
                &[
                    &id,
                    &ev.tenant_id,
                    &ev.hub_id,
                    &driver_sub,
                    &None::<String>,
                    &"app_gps",
                    &quality_name,
                    &loc.lat,
                    &loc.lng,
                    &ev.accuracy_meters,
                    &None::<f64>,
                    &None::<bool>,
                    &ev.time.utc,
                    &ev.time.utc.date_naive().to_string(),
                ],
            )
            .await
            .context("insert app gps observation")?;
        }

        tx.commit().await.context("commit record_event")?;
        Ok(EventReceipt::Accepted {
            server_event_id: ev.event_id.clone(),
        })
    }

    // ---------- bootstrap ----------

    pub async fn ping(&self) -> bool {
        match self.conn().await {
            Ok(client) => client.query_one("SELECT 1", &[]).await.is_ok(),
            Err(_) => false,
        }
    }

    pub async fn ensure_default_org_hub(
        &self,
        config: &crate::config::Config,
    ) -> anyhow::Result<()> {
        let client = self.conn().await?;
        client
            .execute(
                "INSERT INTO organizations (id, name) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                &[&config.default_org_id, &config.default_org_name],
            )
            .await?;
        client
            .execute(
                "INSERT INTO hubs (id, name) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                &[&config.default_hub_id, &config.default_hub_name],
            )
            .await?;
        client
            .execute(
                "INSERT INTO org_hubs (org_id, hub_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                &[&config.default_org_id, &config.default_hub_id],
            )
            .await?;
        Ok(())
    }

    pub async fn link_admin_user(&self, config: &crate::config::Config) -> anyhow::Result<()> {
        let client = self.conn().await?;
        client
            .execute(
                "INSERT INTO users (sub, role_name) VALUES ($1, 'admin') ON CONFLICT DO NOTHING",
                &[&config.default_admin_sub],
            )
            .await?;
        client
            .execute(
                "INSERT INTO user_orgs (user_sub, org_id) \
                 SELECT $1, $2 WHERE EXISTS (SELECT 1 FROM organizations WHERE id = $2) \
                 ON CONFLICT DO NOTHING",
                &[&config.default_admin_sub, &config.default_org_id],
            )
            .await?;
        Ok(())
    }

    // ---------- provisioning ----------

    pub async fn provision_user(
        &self,
        org: &str,
        hub: &str,
        subject: &str,
        display_name: &str,
        role: &str,
    ) -> anyhow::Result<()> {
        let mut client = self.conn().await?;
        let tx = client.transaction().await.context("open provision tx")?;

        let allocated: bool = tx
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM org_hubs WHERE org_id = $1 AND hub_id = $2)",
                &[&org, &hub],
            )
            .await?
            .get(0);
        if !allocated {
            anyhow::bail!("hub {hub} is not allocated to organization {org}");
        }

        tx.execute(
            "INSERT INTO users (sub, display_name, role_name) VALUES ($1,$2,$3) \
             ON CONFLICT (sub) DO UPDATE SET display_name = EXCLUDED.display_name, role_name = EXCLUDED.role_name",
            &[&subject, &display_name, &role],
        )
        .await?;
        tx.execute(
            "INSERT INTO user_orgs (user_sub, org_id) VALUES ($1,$2) ON CONFLICT DO NOTHING",
            &[&subject, &org],
        )
        .await?;
        tx.execute(
            "INSERT INTO user_hubs (user_sub, hub_id) VALUES ($1,$2) ON CONFLICT DO NOTHING",
            &[&subject, &hub],
        )
        .await?;
        tx.commit().await.context("commit provision")?;
        Ok(())
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
        let mut client = self.conn().await?;
        let tx = client.transaction().await.context("open create_hub tx")?;
        let org_exists: bool = tx
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM organizations WHERE id = $1)",
                &[&org],
            )
            .await?
            .get(0);
        if !org_exists {
            return Ok(false);
        }
        tx.execute(
            "INSERT INTO hubs (id, name, lat, lng) VALUES ($1,$2,$3,$4) ON CONFLICT DO NOTHING",
            &[&hub_id, &name, &lat, &lng],
        )
        .await?;
        tx.execute(
            "INSERT INTO org_hubs (org_id, hub_id) VALUES ($1,$2) ON CONFLICT DO NOTHING",
            &[&org, &hub_id],
        )
        .await?;
        tx.commit().await?;
        Ok(true)
    }

    pub async fn update_hub(
        &self,
        org: &str,
        hub_id: &str,
        name: &str,
        lat: f64,
        lng: f64,
    ) -> anyhow::Result<bool> {
        let client = self.conn().await?;
        let n = client
            .execute(
                "UPDATE hubs SET name = $3, lat = $4, lng = $5 \
                 WHERE id = $2 AND EXISTS ( \
                     SELECT 1 FROM org_hubs WHERE org_id = $1 AND hub_id = $2 \
                 )",
                &[&org, &hub_id, &name, &lat, &lng],
            )
            .await?;
        Ok(n > 0)
    }

    pub async fn hub_in_use(&self, org: &str, hub_id: &str) -> anyhow::Result<bool> {
        self.fetch_bool(
            "SELECT EXISTS(SELECT 1 FROM org_hubs WHERE org_id = $1 AND hub_id = $2) AND ( \
                EXISTS(SELECT 1 FROM tasks WHERE hub_id = $2) OR \
                EXISTS(SELECT 1 FROM team_hubs WHERE hub_id = $2) OR \
                EXISTS(SELECT 1 FROM user_hubs WHERE hub_id = $2) OR \
                EXISTS(SELECT 1 FROM hub_vehicles WHERE hub_id = $2) \
             )",
            &[&org, &hub_id],
        )
        .await
    }

    pub async fn delete_hub(&self, org: &str, hub_id: &str) -> anyhow::Result<bool> {
        let mut client = self.conn().await?;
        let tx = client.transaction().await.context("open delete_hub tx")?;
        let n = tx
            .execute(
                "DELETE FROM org_hubs WHERE org_id = $1 AND hub_id = $2",
                &[&org, &hub_id],
            )
            .await?;
        if n == 0 {
            return Ok(false);
        }
        let n = tx
            .execute("DELETE FROM hubs WHERE id = $1", &[&hub_id])
            .await?;
        tx.commit().await?;
        Ok(n > 0)
    }

    // ---------- organization ----------

    pub async fn update_organization(&self, org: &str, name: &str) -> anyhow::Result<bool> {
        let client = self.conn().await?;
        let n = client
            .execute(
                "UPDATE organizations SET name = $2 WHERE id = $1",
                &[&org, &name],
            )
            .await?;
        Ok(n > 0)
    }

    // ---------- teams ----------

    pub async fn teams_for_org(&self, org: &str) -> anyhow::Result<Vec<Value>> {
        self.fetch_json(
            "SELECT jsonb_build_object( \
                'team', row_to_json(t), \
                'hub', th.hub_id, \
                'members', COALESCE(( \
                    SELECT jsonb_agg(row_to_json(u)) \
                    FROM team_members tm JOIN users u ON u.sub = tm.user_sub \
                    WHERE tm.team_id = t.id \
                ), '[]'::jsonb) \
             ) AS data \
             FROM teams t \
             JOIN team_hubs th ON th.team_id = t.id \
             JOIN org_hubs oh ON oh.hub_id = th.hub_id \
             WHERE oh.org_id = $1 \
             ORDER BY t.id",
            &[&org],
        )
        .await
    }

    pub async fn create_team(
        &self,
        org: &str,
        hub_id: &str,
        team_id: &str,
        name: &str,
        shift: &str,
    ) -> anyhow::Result<bool> {
        let mut client = self.conn().await?;
        let tx = client.transaction().await.context("open create_team tx")?;
        let allocated: bool = tx
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM org_hubs WHERE org_id = $1 AND hub_id = $2)",
                &[&org, &hub_id],
            )
            .await?
            .get(0);
        if !allocated {
            return Ok(false);
        }
        tx.execute(
            "INSERT INTO teams (id, name, shift) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING",
            &[&team_id, &name, &shift],
        )
        .await?;
        tx.execute(
            "INSERT INTO team_hubs (team_id, hub_id) VALUES ($1,$2) ON CONFLICT DO NOTHING",
            &[&team_id, &hub_id],
        )
        .await?;
        tx.commit().await?;
        Ok(true)
    }

    pub async fn update_team(
        &self,
        org: &str,
        team_id: &str,
        name: &str,
        shift: &str,
    ) -> anyhow::Result<bool> {
        let client = self.conn().await?;
        let n = client
            .execute(
                "UPDATE teams SET name = $3, shift = $4 \
                 WHERE id = $2 AND EXISTS ( \
                     SELECT 1 FROM team_hubs th \
                     JOIN org_hubs oh ON oh.hub_id = th.hub_id \
                     WHERE th.team_id = $2 AND oh.org_id = $1 \
                 )",
                &[&org, &team_id, &name, &shift],
            )
            .await?;
        Ok(n > 0)
    }

    pub async fn delete_team(&self, org: &str, team_id: &str) -> anyhow::Result<bool> {
        let mut client = self.conn().await?;
        let tx = client.transaction().await.context("open delete_team tx")?;
        tx.execute(
            "DELETE FROM team_members WHERE team_id = $2 AND EXISTS ( \
                SELECT 1 FROM team_hubs th \
                JOIN org_hubs oh ON oh.hub_id = th.hub_id \
                WHERE th.team_id = $2 AND oh.org_id = $1 \
             )",
            &[&org, &team_id],
        )
        .await?;
        tx.execute(
            "DELETE FROM team_hubs WHERE team_id = $2 AND EXISTS ( \
                SELECT 1 FROM org_hubs WHERE org_id = $1 AND hub_id = team_hubs.hub_id \
             )",
            &[&org, &team_id],
        )
        .await?;
        let n = tx
            .execute(
                "DELETE FROM teams WHERE id = $2 AND EXISTS ( \
                    SELECT 1 FROM team_hubs th \
                    JOIN org_hubs oh ON oh.hub_id = th.hub_id \
                    WHERE th.team_id = $2 AND oh.org_id = $1 \
                 )",
                &[&org, &team_id],
            )
            .await?;
        tx.commit().await?;
        Ok(n > 0)
    }

    pub async fn add_team_member(
        &self,
        org: &str,
        team_id: &str,
        subject: &str,
    ) -> anyhow::Result<bool> {
        let client = self.conn().await?;
        let n = client
            .execute(
                "INSERT INTO team_members (team_id, user_sub) \
                 SELECT $2, $3 \
                 WHERE EXISTS ( \
                     SELECT 1 FROM team_hubs th \
                     JOIN org_hubs oh ON oh.hub_id = th.hub_id \
                     WHERE th.team_id = $2 AND oh.org_id = $1 \
                 ) AND EXISTS ( \
                     SELECT 1 FROM user_orgs WHERE org_id = $1 AND user_sub = $3 \
                 ) \
                 ON CONFLICT DO NOTHING",
                &[&org, &team_id, &subject],
            )
            .await?;
        Ok(n > 0)
    }

    pub async fn remove_team_member(
        &self,
        org: &str,
        team_id: &str,
        subject: &str,
    ) -> anyhow::Result<bool> {
        let client = self.conn().await?;
        let n = client
            .execute(
                "DELETE FROM team_members WHERE team_id = $2 AND user_sub = $3 AND EXISTS ( \
                    SELECT 1 FROM team_hubs th \
                    JOIN org_hubs oh ON oh.hub_id = th.hub_id \
                    WHERE th.team_id = $2 AND oh.org_id = $1 \
                 )",
                &[&org, &team_id, &subject],
            )
            .await?;
        Ok(n > 0)
    }

    // ---------- roles ----------

    pub async fn set_cached_role(
        &self,
        org: &str,
        subject: &str,
        role: &str,
    ) -> anyhow::Result<bool> {
        let client = self.conn().await?;
        let n = client
            .execute(
                "UPDATE users SET role_name = $3 \
                 WHERE sub = $2 AND EXISTS ( \
                     SELECT 1 FROM user_orgs WHERE org_id = $1 AND user_sub = $2 \
                 )",
                &[&org, &subject, &role],
            )
            .await?;
        Ok(n > 0)
    }

    // ---------- push tokens ----------

    pub async fn register_push_token(
        &self,
        subject: &str,
        device_id: &str,
        token: &str,
    ) -> anyhow::Result<()> {
        let mut client = self.conn().await?;
        let tx = client
            .transaction()
            .await
            .context("open register_push_token tx")?;

        let existing: Option<String> = tx
            .query_opt(
                "SELECT user_sub FROM user_devices WHERE device_id = $1",
                &[&device_id],
            )
            .await?
            .map(|r| r.get(0));

        match existing {
            Some(other) if other != subject => {
                anyhow::bail!("device {device_id} is already registered to a different user")
            }
            Some(_) => {
                tx.execute(
                    "UPDATE devices SET fcm_token = $2 WHERE id = $1",
                    &[&device_id, &token],
                )
                .await?;
            }
            None => {
                let user_exists: bool = tx
                    .query_one(
                        "SELECT EXISTS(SELECT 1 FROM users WHERE sub = $1)",
                        &[&subject],
                    )
                    .await?
                    .get(0);
                if !user_exists {
                    anyhow::bail!("unknown user {subject}");
                }
                tx.execute(
                    "INSERT INTO devices (id, fcm_token) VALUES ($1,$2) ON CONFLICT DO NOTHING",
                    &[&device_id, &token],
                )
                .await?;
                tx.execute(
                    "INSERT INTO user_devices (user_sub, device_id) VALUES ($1,$2) ON CONFLICT DO NOTHING",
                    &[&subject, &device_id],
                )
                .await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn push_tokens_for_user(&self, subject: &str) -> anyhow::Result<Vec<String>> {
        let client = self.conn().await?;
        let rows = client
            .query(
                "SELECT d.fcm_token FROM devices d \
                 JOIN user_devices ud ON ud.device_id = d.id \
                 WHERE ud.user_sub = $1",
                &[&subject],
            )
            .await?;
        Ok(rows.iter().map(|r| r.get::<usize, String>(0)).collect())
    }

    // ---------- org directories ----------

    pub async fn users_for_org(&self, org_id: &str) -> anyhow::Result<Vec<Value>> {
        self.fetch_json(
            "SELECT jsonb_build_object('user', row_to_json(u)) AS data \
             FROM users u JOIN user_orgs uo ON uo.user_sub = u.sub \
             WHERE uo.org_id = $1 \
             ORDER BY u.sub",
            &[&org_id],
        )
        .await
    }

    pub async fn hubs_for_org(&self, org_id: &str) -> anyhow::Result<Vec<Value>> {
        self.fetch_json(
            "SELECT jsonb_build_object('hub', row_to_json(h)) AS data \
             FROM hubs h JOIN org_hubs oh ON oh.hub_id = h.id \
             WHERE oh.org_id = $1 \
             ORDER BY h.id",
            &[&org_id],
        )
        .await
    }

    pub async fn drivers_for_org(&self, org_id: &str) -> anyhow::Result<Vec<Value>> {
        self.fetch_json(
            "SELECT jsonb_build_object('driver', row_to_json(u)) AS data \
             FROM users u JOIN user_orgs uo ON uo.user_sub = u.sub \
             WHERE uo.org_id = $1 AND u.role_name = 'driver' \
             ORDER BY u.sub",
            &[&org_id],
        )
        .await
    }

    // ---------- costs ----------

    pub async fn record_cost(
        &self,
        entry: &altius_core::CostEntry,
        driver_sub: &str,
    ) -> anyhow::Result<()> {
        let client = self.conn().await?;
        let category = serde_json::to_value(entry.category)?
            .as_str()
            .unwrap_or("other")
            .to_string();
        client
            .execute(
                "INSERT INTO cost_entries (id, driver_sub, category, amount_minor, currency, note, day) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7)",
                &[
                    &entry.id,
                    &driver_sub,
                    &category,
                    &entry.amount_minor,
                    &entry.currency,
                    &entry.note,
                    &entry.day,
                ],
            )
            .await?;
        Ok(())
    }

    pub async fn costs_for_driver(
        &self,
        driver_sub: &str,
        day: Option<&str>,
    ) -> anyhow::Result<Vec<Value>> {
        self.fetch_json(
            "SELECT jsonb_build_object('entry', row_to_json(c)) AS data \
             FROM cost_entries c \
             WHERE c.driver_sub = $1 AND ($2::text IS NULL OR c.day = $2) \
             ORDER BY c.day, c.id",
            &[&driver_sub, &day],
        )
        .await
    }

    // ---------- daily reports ----------

    pub async fn record_daily_report(
        &self,
        report: &altius_core::DailyReport,
        driver_sub: &str,
    ) -> anyhow::Result<()> {
        let client = self.conn().await?;
        let status = serde_json::to_value(report.status)?
            .as_str()
            .unwrap_or("submitted")
            .to_string();
        client
            .execute(
                "INSERT INTO daily_reports \
                 (driver_sub, day, status, revision, hub_id, driver_name, vehicle_number, \
                  odometer_start, odometer_end, notes, visited_task_ids, completed_stop_ids, payload) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
                &[
                    &driver_sub,
                    &report.day,
                    &status,
                    &(report.revision as i64),
                    &report.hub_id,
                    &report.driver_name,
                    &report.vehicle_number,
                    &report.odometer_start.map(|v| v as i64),
                    &report.odometer_end.map(|v| v as i64),
                    &report.notes,
                    &serde_json::to_value(&report.visited_task_ids)?,
                    &serde_json::to_value(&report.completed_stop_ids)?,
                    &serde_json::to_value(report)?,
                ],
            )
            .await?;
        Ok(())
    }

    pub async fn reports_for_driver(&self, driver_sub: &str) -> anyhow::Result<Vec<Value>> {
        self.fetch_json(
            "SELECT jsonb_build_object('report', row_to_json(r)) AS data \
             FROM daily_reports r \
             WHERE r.driver_sub = $1 \
             ORDER BY r.day",
            &[&driver_sub],
        )
        .await
    }

    // ---------- vehicle checks ----------

    pub async fn record_vehicle_check(
        &self,
        org: &str,
        hub: &str,
        check: &altius_core::VehicleCheck,
    ) -> anyhow::Result<()> {
        let client = self.conn().await?;
        let condition = serde_json::to_value(check.condition)?
            .as_str()
            .unwrap_or("good")
            .to_string();
        client
            .execute(
                "INSERT INTO vehicle_checks \
                 (id, driver_sub, org_id, hub_id, day, driver_name, license_plate, vehicle_type, \
                  km_start, km_end, condition, notes, service_date, kir_date, stnk_date, items, payload) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17)",
                &[
                    &check.id,
                    &check.driver_id,
                    &org,
                    &hub,
                    &check.day,
                    &check.driver_name,
                    &check.license_plate,
                    &check.vehicle_type,
                    &(check.km_start as i64),
                    &(check.km_end as i64),
                    &condition,
                    &check.notes,
                    &check.service_date,
                    &check.kir_date,
                    &check.stnk_date,
                    &serde_json::to_value(&check.items)?,
                    &serde_json::to_value(check)?,
                ],
            )
            .await?;
        Ok(())
    }

    pub async fn vehicle_checks_for_org(
        &self,
        org: &str,
        hub: Option<&str>,
    ) -> anyhow::Result<Vec<Value>> {
        self.fetch_json(
            "SELECT jsonb_build_object('check', row_to_json(c), 'driver', c.driver_sub) AS data \
             FROM vehicle_checks c \
             WHERE c.org_id = $1 AND ($2::text IS NULL OR c.hub_id = $2) \
             ORDER BY c.day, c.id",
            &[&org, &hub],
        )
        .await
    }

    // ---------- GPS observations & reviews ----------

    pub async fn record_gps_observation(
        &self,
        obs: &altius_core::GpsObservation,
    ) -> anyhow::Result<()> {
        let client = self.conn().await?;
        let source = serde_json::to_value(obs.source)?
            .as_str()
            .unwrap_or("app_gps")
            .to_string();
        let quality = serde_json::to_value(obs.quality)?
            .as_str()
            .unwrap_or("accurate")
            .to_string();
        let (lat, lng) = obs
            .location
            .map(|c| (Some(c.lat), Some(c.lng)))
            .unwrap_or((None, None));
        client
            .execute(
                "INSERT INTO gps_observations \
                 (id, org_id, hub_id, driver_sub, plate, source, quality, lat, lng, \
                  accuracy_meters, speed_mps, mock_reported, recorded_at, day) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
                &[
                    &obs.id,
                    &obs.tenant_id,
                    &obs.hub_id,
                    &obs.driver_id,
                    &obs.vehicle_id,
                    &source,
                    &quality,
                    &lat,
                    &lng,
                    &obs.accuracy_meters,
                    &obs.speed_mps,
                    &obs.mock_location_reported,
                    &obs.time.utc,
                    &obs.time.utc.date_naive().to_string(),
                ],
            )
            .await?;
        Ok(())
    }

    pub async fn record_gps_review(&self, review: &altius_core::GpsReview) -> anyhow::Result<()> {
        let client = self.conn().await?;
        let classification = serde_json::to_value(review.classification)?
            .as_str()
            .unwrap_or("insufficient_data")
            .to_string();
        let reason = serde_json::to_value(review.reason)?
            .as_str()
            .unwrap_or("stale")
            .to_string();
        client
            .execute(
                "INSERT INTO gps_reviews \
                 (id, org_id, hub_id, driver_sub, plate, app_observation_id, vehicle_observation_id, \
                  classification, reason, separation_meters, time_delta_seconds, reviewed_by, created_at, day) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
                &[
                    &review.id,
                    &review.tenant_id,
                    &review.hub_id,
                    &review.driver_id,
                    &review.vehicle_id,
                    &review.app_observation_id,
                    &review.vehicle_observation_id,
                    &classification,
                    &reason,
                    &review.separation_meters,
                    &review.time_delta_seconds,
                    &review.reviewed_by,
                    &review.created_at,
                    &review.created_at.date_naive().to_string(),
                ],
            )
            .await?;
        Ok(())
    }

    pub async fn mark_gps_review_reviewed(
        &self,
        org: &str,
        review_id: &str,
        reviewer: &str,
    ) -> anyhow::Result<bool> {
        let client = self.conn().await?;
        let n = client
            .execute(
                "UPDATE gps_reviews SET reviewed_by = $3 WHERE org_id = $1 AND id = $2",
                &[&org, &review_id, &reviewer],
            )
            .await?;
        Ok(n > 0)
    }

    pub async fn gps_observations_for_org(
        &self,
        org: &str,
        source: Option<&str>,
        since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> anyhow::Result<Vec<Value>> {
        self.fetch_json(
            "SELECT jsonb_build_object('observation', row_to_json(o)) AS data \
             FROM gps_observations o \
             WHERE o.org_id = $1 \
               AND ($2::text IS NULL OR o.source = $2) \
               AND ($3::timestamptz IS NULL OR o.recorded_at >= $3) \
             ORDER BY o.recorded_at",
            &[&org, &source, &since],
        )
        .await
    }

    pub async fn gps_reviews_for_org(&self, org: &str) -> anyhow::Result<Vec<Value>> {
        self.fetch_json(
            "SELECT jsonb_build_object('review', row_to_json(r)) AS data \
             FROM gps_reviews r \
             WHERE r.org_id = $1 \
             ORDER BY r.created_at",
            &[&org],
        )
        .await
    }

    // ---------- monitoring & McEasy ----------

    pub async fn monitoring_vehicles(&self, org: &str) -> anyhow::Result<Vec<Value>> {
        self.fetch_json(
            "SELECT jsonb_build_object('vehicle', row_to_json(v), 'hub', row_to_json(h)) AS data \
             FROM vehicles v \
             JOIN hub_vehicles hv ON hv.plate = v.plate \
             JOIN hubs h ON h.id = hv.hub_id \
             JOIN org_hubs oh ON oh.hub_id = h.id \
             WHERE oh.org_id = $1 \
             ORDER BY v.plate",
            &[&org],
        )
        .await
    }

    pub async fn organizations(&self) -> anyhow::Result<Vec<String>> {
        let client = self.conn().await?;
        let rows = client
            .query("SELECT id FROM organizations ORDER BY id", &[])
            .await?;
        Ok(rows.iter().map(|r| r.get::<usize, String>(0)).collect())
    }

    pub async fn mceasy_sync_vehicle(
        &self,
        org: &str,
        plate: &str,
        mceasy_id: &str,
    ) -> anyhow::Result<()> {
        let client = self.conn().await?;
        client
            .execute(
                "UPDATE vehicles SET mceasy_vehicle_id = $3 \
                 WHERE plate = $2 AND EXISTS ( \
                     SELECT 1 FROM hub_vehicles hv \
                     JOIN org_hubs oh ON oh.hub_id = hv.hub_id \
                     WHERE hv.plate = $2 AND oh.org_id = $1 \
                 )",
                &[&org, &plate, &mceasy_id],
            )
            .await?;
        Ok(())
    }

    pub async fn mceasy_sync_driver(
        &self,
        org: &str,
        user_sub: &str,
        mceasy_id: &str,
    ) -> anyhow::Result<()> {
        let client = self.conn().await?;
        client
            .execute(
                "UPDATE users SET mceasy_driver_id = $3 \
                 WHERE sub = $2 AND EXISTS ( \
                     SELECT 1 FROM user_orgs WHERE org_id = $1 AND user_sub = $2 \
                 )",
                &[&org, &user_sub, &mceasy_id],
            )
            .await?;
        Ok(())
    }

    pub async fn prune_gps_observations_older_than(
        &self,
        boundary: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<()> {
        let client = self.conn().await?;
        client
            .execute(
                "DELETE FROM gps_observations WHERE recorded_at < $1",
                &[&boundary],
            )
            .await?;
        Ok(())
    }
}
