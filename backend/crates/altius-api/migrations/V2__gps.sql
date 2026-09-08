-- GPS observation and review tables used by the McEasy monitoring worker.

CREATE TABLE IF NOT EXISTS gps_observations (
    id               TEXT PRIMARY KEY,
    org_id           TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    hub_id           TEXT REFERENCES hubs(id) ON DELETE SET NULL,
    driver_sub       TEXT REFERENCES users(sub) ON DELETE SET NULL,
    plate            TEXT REFERENCES vehicles(plate) ON DELETE SET NULL,
    source           TEXT NOT NULL,
    quality          TEXT NOT NULL,
    lat              DOUBLE PRECISION,
    lng              DOUBLE PRECISION,
    accuracy_meters  DOUBLE PRECISION,
    speed_mps        DOUBLE PRECISION,
    mock_reported    BOOLEAN,
    recorded_at      TIMESTAMPTZ NOT NULL,
    day              TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS gps_reviews (
    id                      TEXT PRIMARY KEY,
    org_id                  TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    hub_id                  TEXT REFERENCES hubs(id) ON DELETE SET NULL,
    driver_sub              TEXT REFERENCES users(sub) ON DELETE SET NULL,
    plate                   TEXT REFERENCES vehicles(plate) ON DELETE SET NULL,
    app_observation_id      TEXT REFERENCES gps_observations(id) ON DELETE SET NULL,
    vehicle_observation_id  TEXT REFERENCES gps_observations(id) ON DELETE SET NULL,
    classification          TEXT NOT NULL,
    reason                  TEXT NOT NULL,
    separation_meters       DOUBLE PRECISION,
    time_delta_seconds      DOUBLE PRECISION,
    reviewed_by             TEXT,
    created_at              TIMESTAMPTZ NOT NULL,
    day                     TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_gps_observations_org ON gps_observations(org_id, recorded_at);
CREATE INDEX IF NOT EXISTS idx_gps_observations_driver ON gps_observations(driver_sub, day);
CREATE INDEX IF NOT EXISTS idx_gps_reviews_org ON gps_reviews(org_id, created_at);
CREATE INDEX IF NOT EXISTS idx_gps_reviews_driver ON gps_reviews(driver_sub, day);
