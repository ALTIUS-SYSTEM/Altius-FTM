-- V3 — menutup celah invariant antara skema V1/V2 dan kontrak
-- (packages/api-contracts). Fase EXPAND: semua kolom baru nullable, semua
-- pengetatan kompatibel dengan kode saat ini. Fase CONTRACT (NOT NULL pada
-- kolom tenant baru, PK baru daily_reports) menunggu perubahan kode — lihat
-- Docs/ALTIUS_DATABASE_DESIGN.md §6.
--
-- Catatan refinery: migrasi berjalan di dalam satu transaksi, jadi
-- CREATE INDEX CONCURRENTLY tidak boleh dipakai di sini. Volume tabel
-- masih kecil; ketika tabel besar, build indeks dilakukan di luar refinery.

-- ==========================================================================
-- A. Nilai enum yang hilang / belum dicek
-- ==========================================================================

-- TaskStatusSchema punya 'unassigned'; CHECK lama hanya 4 nilai.
ALTER TABLE tasks
    DROP CONSTRAINT IF EXISTS tasks_stage_check;
ALTER TABLE tasks
    ADD CONSTRAINT tasks_stage_check
    CHECK (stage IN ('unassigned','assigned','in_progress','completed','cancelled'));

ALTER TABLE daily_reports
    ADD CONSTRAINT daily_reports_status_check
    CHECK (status IN ('draft','submitted','revision_requested','approved'));

ALTER TABLE cost_entries
    ADD CONSTRAINT cost_entries_category_check
    CHECK (category IN ('fuel','toll','parking','meal','maintenance','other'));

ALTER TABLE device_events
    ADD CONSTRAINT device_events_action_check
    CHECK (action IN ('arrive','start_activity','complete_activity','depart','skip'));

ALTER TABLE gps_observations
    ADD CONSTRAINT gps_observations_source_check
    CHECK (source IN ('app_gps','vehicle_gps','manual')),
    ADD CONSTRAINT gps_observations_quality_check
    CHECK (quality IN ('accurate','degraded','unavailable'));

ALTER TABLE gps_reviews
    ADD CONSTRAINT gps_reviews_classification_check
    CHECK (classification IN ('consistent','review_required','insufficient_data')),
    ADD CONSTRAINT gps_reviews_reason_check
    CHECK (reason IN ('within_tolerance','separation','missing_pair','poor_accuracy','stale','scope_mismatch'));

ALTER TABLE vehicle_checks
    ADD CONSTRAINT vehicle_checks_condition_check
    CHECK (condition IN ('good','not_good')),
    ADD CONSTRAINT vehicle_checks_km_order
    CHECK (km_end IS NULL OR km_start IS NULL OR km_end >= km_start);

-- ==========================================================================
-- B. Format kalender `day` — DateOnlySchema: 'YYYY-MM-DD'
-- ==========================================================================

ALTER TABLE tasks            ADD CONSTRAINT tasks_day_check            CHECK (day ~ '^\d{4}-\d{2}-\d{2}$');
ALTER TABLE device_events    ADD CONSTRAINT device_events_day_check    CHECK (day ~ '^\d{4}-\d{2}-\d{2}$');
ALTER TABLE daily_reports    ADD CONSTRAINT daily_reports_day_check    CHECK (day ~ '^\d{4}-\d{2}-\d{2}$');
ALTER TABLE cost_entries     ADD CONSTRAINT cost_entries_day_check     CHECK (day ~ '^\d{4}-\d{2}-\d{2}$');
ALTER TABLE vehicle_checks   ADD CONSTRAINT vehicle_checks_day_check   CHECK (day ~ '^\d{4}-\d{2}-\d{2}$');
ALTER TABLE gps_observations ADD CONSTRAINT gps_observations_day_check CHECK (day ~ '^\d{4}-\d{2}-\d{2}$');
ALTER TABLE gps_reviews      ADD CONSTRAINT gps_reviews_day_check      CHECK (day ~ '^\d{4}-\d{2}-\d{2}$');

-- ==========================================================================
-- C. Range koordinat — CoordinateSchema: lat [-90,90], lng [-180,180]
-- ==========================================================================

ALTER TABLE hubs
    ADD CONSTRAINT hubs_lat_range CHECK (lat IS NULL OR lat BETWEEN -90 AND 90),
    ADD CONSTRAINT hubs_lng_range CHECK (lng IS NULL OR lng BETWEEN -180 AND 180);
ALTER TABLE stops
    ADD CONSTRAINT stops_lat_range CHECK (lat IS NULL OR lat BETWEEN -90 AND 90),
    ADD CONSTRAINT stops_lng_range CHECK (lng IS NULL OR lng BETWEEN -180 AND 180);
ALTER TABLE gps_observations
    ADD CONSTRAINT gps_obs_lat_range CHECK (lat IS NULL OR lat BETWEEN -90 AND 90),
    ADD CONSTRAINT gps_obs_lng_range CHECK (lng IS NULL OR lng BETWEEN -180 AND 180);

-- ==========================================================================
-- D. Invariant GPS — GpsObservationSchema
-- ==========================================================================

-- vehicle_gps harus menamai kendaraan sumbernya (independensi dua stream).
ALTER TABLE gps_observations
    ADD CONSTRAINT gps_obs_vehicle_requires_plate
    CHECK (source <> 'vehicle_gps' OR plate IS NOT NULL),
    -- quality 'unavailable' tidak boleh membawa fix.
    ADD CONSTRAINT gps_obs_unavailable_has_no_fix
    CHECK (quality <> 'unavailable' OR (lat IS NULL AND lng IS NULL AND accuracy_meters IS NULL));
-- BELUM ditegakkan (arah sebaliknya): non-'unavailable' mewajibkan
-- position+accuracy — record_event saat ini menulis 'accurate' dengan
-- accuracy_meters NULL. Perlu perbaikan kode lebih dulu.

-- ==========================================================================
-- E. Urutan stop unik per task — TaskSchema superRefine
-- ==========================================================================

ALTER TABLE stops
    ADD CONSTRAINT stops_task_sequence_unique UNIQUE (task_id, sequence);

-- ==========================================================================
-- F. Kunci tenant yang hilang (EXPAND — NOT NULL menyusul di V4)
-- ==========================================================================

-- Event membawa hub_id di kontrak dan divalidasi di route, tapi tidak
-- disimpan. Simpan agar event bisa di-query per hub.
ALTER TABLE device_events
    ADD COLUMN hub_id TEXT REFERENCES hubs(id) ON DELETE SET NULL;
UPDATE device_events de SET hub_id = t.hub_id
    FROM tasks t WHERE de.task_id = t.id AND de.hub_id IS NULL;

-- ExpenseSchema mewajibkan tenantId+hubId. Tanpa org_id, biaya seorang
-- driver multi-org bocor lintas tenant (costs_for_driver tak ber-scope org).
ALTER TABLE cost_entries
    ADD COLUMN org_id TEXT REFERENCES organizations(id) ON DELETE CASCADE,
    ADD COLUMN hub_id TEXT REFERENCES hubs(id) ON DELETE SET NULL;
UPDATE cost_entries c SET org_id = (
    SELECT uo.org_id FROM user_orgs uo
    WHERE uo.user_sub = c.driver_sub ORDER BY uo.org_id LIMIT 1
) WHERE c.org_id IS NULL;

-- LhsReportSchema mewajibkan tenantId. PK (driver_sub, day) ambigu untuk
-- driver multi-org; V4 memindahkan PK ke (org_id, driver_sub, day).
ALTER TABLE daily_reports
    ADD COLUMN org_id TEXT REFERENCES organizations(id) ON DELETE CASCADE;
UPDATE daily_reports r SET org_id = (
    SELECT uo.org_id FROM user_orgs uo
    WHERE uo.user_sub = r.driver_sub ORDER BY uo.org_id LIMIT 1
) WHERE r.org_id IS NULL;

-- ==========================================================================
-- G. Assignee harus anggota org task — sekarang tidak dicek di mana pun
--    (create_task/update_task hanya memeriksa alokasi hub). Ditegakkan
--    lewat FK komposit; bukan denormalisasi performa, melainkan cara
--    relasional mengekspresikan invariant ini.
-- ==========================================================================

ALTER TABLE tasks     ADD CONSTRAINT tasks_id_org_unique      UNIQUE (id, org_id);
ALTER TABLE user_orgs ADD CONSTRAINT user_orgs_org_user_unique UNIQUE (org_id, user_sub);

ALTER TABLE task_assignments ADD COLUMN org_id TEXT;
UPDATE task_assignments ta SET org_id = t.org_id
    FROM tasks t WHERE ta.task_id = t.id;
-- Pre-flight untuk data lama: baris yang melanggar akan menggagalkan FK —
--   SELECT ta.task_id, ta.driver_sub FROM task_assignments ta
--   JOIN tasks t ON t.id = ta.task_id
--   WHERE NOT EXISTS (SELECT 1 FROM user_orgs uo
--                     WHERE uo.org_id = t.org_id AND uo.user_sub = ta.driver_sub);
ALTER TABLE task_assignments
    DROP CONSTRAINT task_assignments_task_id_fkey,
    DROP CONSTRAINT task_assignments_driver_sub_fkey,
    ADD CONSTRAINT task_assignments_task_fk
        FOREIGN KEY (task_id, org_id) REFERENCES tasks(id, org_id) ON DELETE CASCADE,
    ADD CONSTRAINT task_assignments_member_fk
        FOREIGN KEY (org_id, driver_sub) REFERENCES user_orgs(org_id, user_sub) ON DELETE CASCADE;
-- org_id sengaja dibiarkan nullable di V3: INSERT saat ini tidak mengisi
-- kolomnya. SET NOT NULL di V4 setelah kode meneruskan org.

-- ==========================================================================
-- H. Perilaku delete yang disengaja
-- ==========================================================================

-- Hapus hub tidak boleh menghapus task. App sudah memeriksa hub_in_use;
-- RESTRICT menjadikannya invariant DB, bukan harapan.
ALTER TABLE tasks
    DROP CONSTRAINT tasks_hub_id_fkey,
    ADD CONSTRAINT tasks_hub_id_fkey
        FOREIGN KEY (hub_id) REFERENCES hubs(id) ON DELETE RESTRICT;

-- Report/check/review adalah catatan audit: hub-nya tidak boleh hilang.
ALTER TABLE daily_reports
    ALTER COLUMN hub_id SET NOT NULL,
    DROP CONSTRAINT daily_reports_hub_id_fkey,
    ADD CONSTRAINT daily_reports_hub_id_fkey
        FOREIGN KEY (hub_id) REFERENCES hubs(id) ON DELETE RESTRICT;
ALTER TABLE vehicle_checks
    ALTER COLUMN hub_id SET NOT NULL,
    DROP CONSTRAINT vehicle_checks_hub_id_fkey,
    ADD CONSTRAINT vehicle_checks_hub_id_fkey
        FOREIGN KEY (hub_id) REFERENCES hubs(id) ON DELETE RESTRICT;
ALTER TABLE gps_reviews
    ALTER COLUMN hub_id SET NOT NULL,
    DROP CONSTRAINT gps_reviews_hub_id_fkey,
    ADD CONSTRAINT gps_reviews_hub_id_fkey
        FOREIGN KEY (hub_id) REFERENCES hubs(id) ON DELETE RESTRICT;
-- Kode: hub_in_use harus memeriksa daily_reports, vehicle_checks, dan
-- gps_reviews juga — kalau tidak, delete_hub gagal di FK dengan error
-- yang tidak di-mapping ke 4xx.

-- Observasi GPS diprune setelah 72 jam, tapi review adalah audit yang
-- berumur panjang. SET NULL akan menghapus referensi historis saat prune;
-- jatuhkan FK — id observasi tetap tersimpan sebagai referensi tekstual.
ALTER TABLE gps_reviews
    DROP CONSTRAINT gps_reviews_app_observation_id_fkey,
    DROP CONSTRAINT gps_reviews_vehicle_observation_id_fkey;

-- ==========================================================================
-- I. NOT NULL sesuai kontrak (tipe Rust menjamin non-null di semua writer)
-- ==========================================================================

ALTER TABLE device_events  ALTER COLUMN device_id      SET NOT NULL;
ALTER TABLE vehicle_checks ALTER COLUMN driver_name    SET NOT NULL,
                           ALTER COLUMN license_plate  SET NOT NULL,
                           ALTER COLUMN vehicle_type   SET NOT NULL,
                           ALTER COLUMN km_start       SET NOT NULL,
                           ALTER COLUMN km_end         SET NOT NULL,
                           ALTER COLUMN condition      SET NOT NULL,
                           ALTER COLUMN notes          SET NOT NULL;
ALTER TABLE daily_reports  ALTER COLUMN driver_name    SET NOT NULL,
                           ALTER COLUMN vehicle_number SET NOT NULL,
                           ALTER COLUMN notes          SET NOT NULL;

-- Kontrak mewajibkan field ini; kolom ditambah sekarang, pengisian
-- menyusul lewat perubahan kode provisioning.
ALTER TABLE organizations ADD COLUMN time_zone TEXT NOT NULL DEFAULT 'UTC';
ALTER TABLE hubs          ADD COLUMN time_zone TEXT NOT NULL DEFAULT 'UTC';
ALTER TABLE users         ADD COLUMN email TEXT;

-- ==========================================================================
-- J. Lebar integer — Rust mengikat u32 sebagai i64; int4 menolaknya
--    (latent bug: tidak pernah tercakup karena pg_it selalu skip tanpa
--    TEST_DATABASE_URL). BIGINT juga menampung rentang penuh u32.
-- ==========================================================================

ALTER TABLE stops
    ALTER COLUMN sequence TYPE BIGINT,
    ALTER COLUMN service_seconds TYPE BIGINT;
ALTER TABLE daily_reports
    ALTER COLUMN revision TYPE BIGINT,
    ALTER COLUMN odometer_start TYPE BIGINT,
    ALTER COLUMN odometer_end TYPE BIGINT;
ALTER TABLE vehicle_checks
    ALTER COLUMN km_start TYPE BIGINT,
    ALTER COLUMN km_end TYPE BIGINT;

-- ==========================================================================
-- K. Indeks — tiap indeks punya query atau operasi FK yang membenarkannya
-- ==========================================================================

-- tasks_for_org: WHERE t.org_id = $1 ORDER BY t.id — filter + urut
-- dalam satu indeks; menggantikan idx_tasks_org.
DROP INDEX IF EXISTS idx_tasks_org;
CREATE INDEX idx_tasks_org_id ON tasks(org_id, id);

-- Subsumed oleh UNIQUE(task_id, sequence) di bagian E.
DROP INDEX IF EXISTS idx_stops_task;

-- Join org_hubs yang dipimpin hub_id (record_event, update_*,
-- teams_for_org, mceasy_sync_vehicle): PK (org_id, hub_id) tidak
-- melayani pencarian dari sisi hub.
CREATE INDEX idx_org_hubs_hub ON org_hubs(hub_id, org_id);
-- PK (org_id, hub_id) sudah menutup idx_org_hubs_org.
DROP INDEX IF EXISTS idx_org_hubs_org;

-- register_push_token: SELECT ... WHERE device_id = $1 — device_id adalah
-- kolom kedua PK (user_sub, device_id).
CREATE INDEX idx_user_devices_device ON user_devices(device_id);
DROP INDEX IF EXISTS idx_user_devices_user; -- redundant: prefix PK

-- mceasy_sync_vehicle: EXISTS(... WHERE hv.plate = $2 ...) — plate adalah
-- kolom kedua PK (hub_id, plate).
CREATE INDEX idx_hub_vehicles_plate ON hub_vehicles(plate, hub_id);
DROP INDEX IF EXISTS idx_hub_vehicles_hub; -- redundant: prefix PK

-- Indeks di sisi mereferensi untuk ON DELETE / SET NULL (aturan
-- FK-index): tanpa ini, penghapusan parent memicu seq scan.
CREATE INDEX idx_device_events_stop      ON device_events(stop_id);
CREATE INDEX idx_team_members_user       ON team_members(user_sub);
CREATE INDEX idx_user_hubs_hub           ON user_hubs(hub_id);
CREATE INDEX idx_daily_reports_hub       ON daily_reports(hub_id);
CREATE INDEX idx_vehicle_checks_hub      ON vehicle_checks(hub_id);
CREATE INDEX idx_vehicle_checks_driver   ON vehicle_checks(driver_sub);
CREATE INDEX idx_gps_observations_hub    ON gps_observations(hub_id);
CREATE INDEX idx_gps_observations_plate  ON gps_observations(plate);
CREATE INDEX idx_gps_reviews_hub         ON gps_reviews(hub_id);
-- driver_sub sudah tertutup oleh idx_gps_reviews_driver (driver_sub, day)
-- dan idx_gps_observations_driver (driver_sub, day) dari V2.
CREATE INDEX idx_gps_reviews_plate       ON gps_reviews(plate);
-- Cascade dari user_orgs(org_id, user_sub) → task_assignments.
CREATE INDEX idx_task_assignments_member ON task_assignments(org_id, driver_sub);

-- prune_gps_observations_older_than: DELETE ... WHERE recorded_at < $1
-- bersifat lintas-org; indeks (org_id, recorded_at) tidak melayaninya.
CREATE INDEX idx_gps_observations_recorded ON gps_observations(recorded_at);

-- Indeks yang sepenuhnya redundant terhadap PK (biaya tulis tanpa manfaat):
DROP INDEX IF EXISTS idx_user_orgs_user;      -- prefix PK (user_sub, org_id)
DROP INDEX IF EXISTS idx_user_hubs_user;      -- prefix PK (user_sub, hub_id)
DROP INDEX IF EXISTS idx_daily_reports_driver; -- sama dengan PK (driver_sub, day)
