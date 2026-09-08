-- Altius FTM core schema.
-- All tenant-scoped tables carry `org_id` (or are reachable through `org_id`)
-- so scoping is a WHERE clause, not a traversal.

CREATE TABLE IF NOT EXISTS organizations (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS hubs (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    lat         DOUBLE PRECISION,
    lng         DOUBLE PRECISION
);

CREATE TABLE IF NOT EXISTS users (
    sub               TEXT PRIMARY KEY,
    display_name      TEXT,
    role_name         TEXT,
    mceasy_driver_id  TEXT
);

CREATE TABLE IF NOT EXISTS user_orgs (
    user_sub  TEXT NOT NULL REFERENCES users(sub) ON DELETE CASCADE,
    org_id    TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    PRIMARY KEY (user_sub, org_id)
);

CREATE TABLE IF NOT EXISTS user_hubs (
    user_sub  TEXT NOT NULL REFERENCES users(sub) ON DELETE CASCADE,
    hub_id    TEXT NOT NULL REFERENCES hubs(id) ON DELETE CASCADE,
    PRIMARY KEY (user_sub, hub_id)
);

CREATE TABLE IF NOT EXISTS org_hubs (
    org_id  TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    hub_id  TEXT NOT NULL REFERENCES hubs(id) ON DELETE CASCADE,
    PRIMARY KEY (org_id, hub_id)
);

CREATE TABLE IF NOT EXISTS teams (
    id    TEXT PRIMARY KEY,
    name  TEXT NOT NULL,
    shift TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS team_hubs (
    team_id  TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    hub_id   TEXT NOT NULL REFERENCES hubs(id) ON DELETE CASCADE,
    PRIMARY KEY (team_id, hub_id)
);

CREATE TABLE IF NOT EXISTS team_members (
    team_id  TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    user_sub TEXT NOT NULL REFERENCES users(sub) ON DELETE CASCADE,
    PRIMARY KEY (team_id, user_sub)
);

CREATE TABLE IF NOT EXISTS vehicles (
    plate               TEXT PRIMARY KEY,
    display_name        TEXT,
    mceasy_vehicle_id   TEXT,
    mceasy_last_seen    TIMESTAMPTZ,
    mceasy_ignition     BOOLEAN
);

CREATE TABLE IF NOT EXISTS hub_vehicles (
    hub_id  TEXT NOT NULL REFERENCES hubs(id) ON DELETE CASCADE,
    plate   TEXT NOT NULL REFERENCES vehicles(plate) ON DELETE CASCADE,
    PRIMARY KEY (hub_id, plate)
);

CREATE TABLE IF NOT EXISTS tasks (
    id          TEXT PRIMARY KEY,
    org_id      TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    hub_id      TEXT NOT NULL REFERENCES hubs(id) ON DELETE CASCADE,
    title       TEXT NOT NULL,
    stage       TEXT NOT NULL DEFAULT 'assigned' CHECK (stage IN ('assigned','in_progress','completed','cancelled')),
    day         TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS task_assignments (
    task_id    TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    driver_sub TEXT NOT NULL REFERENCES users(sub) ON DELETE CASCADE,
    PRIMARY KEY (task_id, driver_sub)
);

CREATE TABLE IF NOT EXISTS stops (
    id               TEXT PRIMARY KEY,
    task_id          TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    sequence         INTEGER NOT NULL,
    name             TEXT NOT NULL,
    address          TEXT NOT NULL,
    lat              DOUBLE PRECISION,
    lng              DOUBLE PRECISION,
    stage            TEXT NOT NULL DEFAULT 'pending' CHECK (stage IN ('pending','arrived','working','completed','departed','skipped')),
    service_seconds  INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS devices (
    id         TEXT PRIMARY KEY,
    fcm_token  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS user_devices (
    user_sub   TEXT NOT NULL REFERENCES users(sub) ON DELETE CASCADE,
    device_id  TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    PRIMARY KEY (user_sub, device_id)
);

CREATE TABLE IF NOT EXISTS device_events (
    id            TEXT PRIMARY KEY,
    org_id        TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    driver_sub    TEXT NOT NULL REFERENCES users(sub) ON DELETE CASCADE,
    device_id     TEXT,
    task_id       TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    stop_id       TEXT REFERENCES stops(id) ON DELETE SET NULL,
    request_key   TEXT NOT NULL,
    action        TEXT NOT NULL,
    occurred_utc  TIMESTAMPTZ NOT NULL,
    offset_min    INTEGER NOT NULL CHECK (offset_min BETWEEN -840 AND 840),
    day           TEXT NOT NULL,
    payload       JSONB NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (org_id, driver_sub, request_key)
);

CREATE TABLE IF NOT EXISTS daily_reports (
    driver_sub          TEXT NOT NULL REFERENCES users(sub) ON DELETE CASCADE,
    day                 TEXT NOT NULL,
    status              TEXT NOT NULL DEFAULT 'submitted',
    revision            INTEGER NOT NULL DEFAULT 0,
    hub_id              TEXT REFERENCES hubs(id) ON DELETE SET NULL,
    driver_name         TEXT,
    vehicle_number      TEXT,
    odometer_start      INTEGER,
    odometer_end        INTEGER,
    notes               TEXT,
    visited_task_ids    JSONB NOT NULL DEFAULT '[]',
    completed_stop_ids  JSONB NOT NULL DEFAULT '[]',
    payload             JSONB NOT NULL,
    PRIMARY KEY (driver_sub, day)
);

CREATE TABLE IF NOT EXISTS cost_entries (
    id            TEXT PRIMARY KEY,
    driver_sub    TEXT NOT NULL REFERENCES users(sub) ON DELETE CASCADE,
    category      TEXT NOT NULL,
    amount_minor  BIGINT NOT NULL CHECK (amount_minor >= 0 AND amount_minor <= 1000000000000),
    currency      TEXT NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    note          TEXT NOT NULL DEFAULT '',
    day           TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS vehicle_checks (
    id              TEXT PRIMARY KEY,
    driver_sub      TEXT NOT NULL REFERENCES users(sub) ON DELETE CASCADE,
    org_id          TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    hub_id          TEXT REFERENCES hubs(id) ON DELETE SET NULL,
    day             TEXT NOT NULL,
    driver_name     TEXT,
    license_plate   TEXT,
    vehicle_type    TEXT,
    km_start        INTEGER,
    km_end          INTEGER,
    condition       TEXT,
    notes           TEXT,
    service_date    TEXT,
    kir_date        TEXT,
    stnk_date       TEXT,
    items           JSONB NOT NULL DEFAULT '[]',
    payload         JSONB NOT NULL
);

-- Indexes for the hot tenant-scoped lookups.
CREATE INDEX IF NOT EXISTS idx_tasks_org ON tasks(org_id);
CREATE INDEX IF NOT EXISTS idx_tasks_hub ON tasks(hub_id);
CREATE INDEX IF NOT EXISTS idx_stops_task ON stops(task_id);
CREATE INDEX IF NOT EXISTS idx_device_events_org ON device_events(org_id);
CREATE INDEX IF NOT EXISTS idx_device_events_driver ON device_events(driver_sub, day);
CREATE INDEX IF NOT EXISTS idx_device_events_task ON device_events(task_id);
CREATE INDEX IF NOT EXISTS idx_daily_reports_driver ON daily_reports(driver_sub, day);
CREATE INDEX IF NOT EXISTS idx_cost_entries_driver ON cost_entries(driver_sub, day);
CREATE INDEX IF NOT EXISTS idx_vehicle_checks_org ON vehicle_checks(org_id, day);
CREATE INDEX IF NOT EXISTS idx_user_orgs_user ON user_orgs(user_sub);
CREATE INDEX IF NOT EXISTS idx_user_hubs_user ON user_hubs(user_sub);
CREATE INDEX IF NOT EXISTS idx_org_hubs_org ON org_hubs(org_id);
CREATE INDEX IF NOT EXISTS idx_team_hubs_hub ON team_hubs(hub_id);
CREATE INDEX IF NOT EXISTS idx_team_members_team ON team_members(team_id);
CREATE INDEX IF NOT EXISTS idx_hub_vehicles_hub ON hub_vehicles(hub_id);
CREATE INDEX IF NOT EXISTS idx_user_devices_user ON user_devices(user_sub);
