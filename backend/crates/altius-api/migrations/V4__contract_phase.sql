-- V4 — CONTRACT phase of the V3 expand/contract plan.
--
-- Deploy order (expand → backfill → contract):
--   1. Ship Phase 0 app code that dual-writes the tenant / GPS / email columns
--      (create_task org_id on assignments, record_event hub_id + GPS accuracy,
--      record_cost / record_daily_report tenant cols, provision_user email).
--   2. Run this migration only after that deploy has written new rows correctly.
--   3. SET NOT NULL / PK change / CHECKs below then lock the contract.
--
-- Prerequisites (must already be true in application code before migrate):
--   * create_task / update_task write task_assignments.org_id
--   * record_event writes device_events.hub_id
--   * record_cost / record_daily_report write tenant columns
--   * provision_user writes users.email when provided
--   * record_event GPS path only writes accurate with accuracy present
--
-- Order inside this file: ADD nullable columns → backfill → SET NOT NULL /
-- PK change → CHECKs. Do not reorder. Refinery wraps the file in one transaction.

-- ==========================================================================
-- A. New contract columns on device_events (nullable — old devices omit them)
-- ==========================================================================

ALTER TABLE device_events
    ADD COLUMN IF NOT EXISTS schema_version INTEGER,
    ADD COLUMN IF NOT EXISTS device_sequence BIGINT,
    ADD COLUMN IF NOT EXISTS expected_task_revision BIGINT,
    ADD COLUMN IF NOT EXISTS reason TEXT,
    ADD COLUMN IF NOT EXISTS observation_id TEXT;

-- App-level sync_events rejects skip without reason; this CHECK is the DB belt.
ALTER TABLE device_events
    DROP CONSTRAINT IF EXISTS device_events_skip_requires_reason;
ALTER TABLE device_events
    ADD CONSTRAINT device_events_skip_requires_reason
    CHECK (action <> 'skip' OR reason IS NOT NULL);

-- ==========================================================================
-- B. Backfill tenant columns that V3 left nullable
-- ==========================================================================

-- Assignments inherit the task's org.
UPDATE task_assignments ta
SET org_id = t.org_id
FROM tasks t
WHERE ta.task_id = t.id AND ta.org_id IS NULL;

-- Events inherit hub from the related task when the writer omitted it.
UPDATE device_events de
SET hub_id = t.hub_id
FROM tasks t
WHERE de.task_id = t.id AND de.hub_id IS NULL;

-- Orphan events (task deleted / never linked) take any hub in the same org.
-- Last-resort only — prefer failing the DO block below over inventing tenants.
UPDATE device_events de
SET hub_id = (
    SELECT oh.hub_id FROM org_hubs oh WHERE oh.org_id = de.org_id LIMIT 1
)
WHERE de.hub_id IS NULL;

-- Costs: org from the driver's membership; hub from user_hubs or org default.
UPDATE cost_entries c
SET org_id = uo.org_id
FROM user_orgs uo
WHERE c.driver_sub = uo.user_sub AND c.org_id IS NULL;

UPDATE cost_entries c
SET hub_id = uh.hub_id
FROM user_hubs uh
WHERE c.driver_sub = uh.user_sub AND c.hub_id IS NULL;

UPDATE cost_entries c
SET hub_id = (
    SELECT oh.hub_id FROM org_hubs oh WHERE oh.org_id = c.org_id LIMIT 1
)
WHERE c.hub_id IS NULL AND c.org_id IS NOT NULL;

-- Daily reports: org from membership when missing.
UPDATE daily_reports r
SET org_id = uo.org_id
FROM user_orgs uo
WHERE r.driver_sub = uo.user_sub AND r.org_id IS NULL;

-- users.email: placeholder for rows never provisioned with an address.
-- Operators should replace these; NOT NULL below needs a non-null value.
UPDATE users
SET email = sub || '@users.invalid'
WHERE email IS NULL OR btrim(email) = '';

-- Fail loudly if backfill could not satisfy NOT NULL (no membership / no hub).
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM task_assignments WHERE org_id IS NULL) THEN
        RAISE EXCEPTION 'V4: task_assignments.org_id still NULL after backfill';
    END IF;
    IF EXISTS (SELECT 1 FROM device_events WHERE hub_id IS NULL) THEN
        RAISE EXCEPTION 'V4: device_events.hub_id still NULL after backfill';
    END IF;
    IF EXISTS (SELECT 1 FROM cost_entries WHERE org_id IS NULL OR hub_id IS NULL) THEN
        RAISE EXCEPTION 'V4: cost_entries tenant columns still NULL after backfill';
    END IF;
    IF EXISTS (SELECT 1 FROM daily_reports WHERE org_id IS NULL) THEN
        RAISE EXCEPTION 'V4: daily_reports.org_id still NULL after backfill';
    END IF;
END $$;

-- ==========================================================================
-- C. SET NOT NULL + daily_reports PK → (org_id, driver_sub, day)
-- ==========================================================================

ALTER TABLE task_assignments ALTER COLUMN org_id SET NOT NULL;
ALTER TABLE device_events ALTER COLUMN hub_id SET NOT NULL;
ALTER TABLE cost_entries ALTER COLUMN org_id SET NOT NULL;
ALTER TABLE cost_entries ALTER COLUMN hub_id SET NOT NULL;
ALTER TABLE daily_reports ALTER COLUMN org_id SET NOT NULL;
ALTER TABLE users ALTER COLUMN email SET NOT NULL;

ALTER TABLE daily_reports DROP CONSTRAINT IF EXISTS daily_reports_pkey;
ALTER TABLE daily_reports
    ADD CONSTRAINT daily_reports_pkey PRIMARY KEY (org_id, driver_sub, day);

-- ==========================================================================
-- D. GPS contract CHECKs (INV-13 / INV-14) — code path must already comply
-- ==========================================================================

ALTER TABLE gps_observations
    DROP CONSTRAINT IF EXISTS gps_obs_accurate_requires_fix;
ALTER TABLE gps_observations
    ADD CONSTRAINT gps_obs_accurate_requires_fix
    CHECK (
        quality <> 'accurate'
        OR (lat IS NOT NULL AND lng IS NOT NULL AND accuracy_meters IS NOT NULL)
    );

ALTER TABLE gps_observations
    DROP CONSTRAINT IF EXISTS gps_obs_app_gps_mock_reported;
ALTER TABLE gps_observations
    ADD CONSTRAINT gps_obs_app_gps_mock_reported
    CHECK (
        source <> 'app_gps'
        OR lat IS NULL
        OR mock_reported IS NOT NULL
    );

-- ==========================================================================
-- E. Supporting indexes for retention prune (worker lands in a later change)
-- ==========================================================================

CREATE INDEX IF NOT EXISTS idx_device_events_occurred
    ON device_events (occurred_utc);
