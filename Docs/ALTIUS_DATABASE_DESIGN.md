# Altius FTM — Database Design (PostgreSQL)

> **Product:** Altius FTM (Fleet & Transport Management)  
> **Source of truth:** `backend/crates/altius-api/migrations/` (refinery, applied by `PgStore::migrate`)  
> **Domain contracts:** `packages/api-contracts` (Zod) ↔ `backend/crates/altius-core` (Rust)  
> **Date:** 2026-09-08 — V1__init, V2__gps, V3__contract_integrity  

Dokumen ini adalah desain skema PostgreSQL untuk Altius FTM. Invariant didefinisikan
oleh kontrak API (`api-contracts`); skema menegakkannya di level database, bukan
hanya di kode.

---

## 1. Model Entitas

### 1.1 Identitas & tenancy

| Tabel | Kolom penting | Tipe / catatan |
|---|---|---|
| `organizations` | `id` PK, `name`, `time_zone`, `created_at` | `time_zone` IANA, `DEFAULT 'UTC'` (V3) — kontrak `OrganizationSchema` mewajibkannya |
| `hubs` | `id` PK, `name`, `lat`, `lng`, `time_zone` | `lat`/`lng` nullable dengan CHECK range (V3); `time_zone NOT NULL DEFAULT 'UTC'` |
| `users` | `sub` PK (Keycloak subject), `display_name`, `role_name` (cache peran primer), `mceasy_driver_id`, `email` (V3) | `sub` adalah FK identitas — bukan email; `role_name` adalah cache, sumber kebenaran peran = Keycloak token + `user_orgs`/`user_hubs` membership |
| `user_orgs` | `(user_sub, org_id)` PK | Keanggotaan user↔org; multi-org diizinkan kontrak (`MembershipSchema[]`) |
| `user_hubs` | `(user_sub, hub_id)` PK | Penempatan user↔hub |
| `org_hubs` | `(org_id, hub_id)` PK | Alokasi hub↔org — **kunci isolasi tenant**; semua query scoped lewat sini |
| `devices` | `id` PK, `fcm_token` | Token FCM per device id |
| `user_devices` | `(user_sub, device_id)` PK | Relasi user↔device |

### 1.2 Operasional

| Tabel | Kolom penting | Catatan |
|---|---|---|
| `teams` | `id` PK, `name`, `shift` | Di-scope lewat `team_hubs`→`org_hubs` |
| `team_hubs` | `(team_id, hub_id)` PK | — |
| `team_members` | `(team_id, user_sub)` PK | — |
| `vehicles` | `plate` PK (natural key), `display_name`, `mceasy_vehicle_id`, `mceasy_last_seen`, `mceasy_ignition` | Plate dipakai sebagai identitas kendaraan di kontrak (`vehicleId` = plate) |
| `hub_vehicles` | `(hub_id, plate)` PK | — |
| `tasks` | `id` PK, `org_id`, `hub_id`, `title`, `stage` (enum CHECK), `day`, `created_at` | `stage` kini mencakup `unassigned` (V3) |
| `task_assignments` | `(task_id, driver_sub)` PK + `org_id` (V3) | FK komposit V3: `(task_id, org_id)→tasks`, `(org_id, driver_sub)→user_orgs` — assignee **wajib anggota org task** di level DB |
| `stops` | `id` PK, `task_id`, `sequence` BIGINT, `name`, `address`, `lat`, `lng`, `stage` (enum CHECK), `service_seconds` BIGINT | `UNIQUE(task_id, sequence)` (V3) — kontrak melarang sequence duplikat |

### 1.3 Event & audit

| Tabel | Kolom penting | Catatan |
|---|---|---|
| `device_events` | `id` PK, `org_id`, `driver_sub`, `device_id` NOT NULL (V3), `task_id` (nullable, SET NULL), `stop_id` (nullable, SET NULL), `hub_id` (V3, nullable), `request_key`, `action` (enum CHECK), `occurred_utc`, `offset_min`, `day`, `payload` JSONB, `created_at` | `UNIQUE(org_id, driver_sub, request_key)` = idempotency; append-only |
| `daily_reports` (LHS) | `(driver_sub, day)` PK — V4: `(org_id, driver_sub, day)`, `status` enum CHECK, `revision`, `hub_id` NOT NULL + RESTRICT (V3), `org_id` (V3), `odometer_*`, `visited_task_ids`/`completed_stop_ids` JSONB, `payload` JSONB | Status/revision server-owned (`LhsReportRequestSchema` membuangnya dari body) |
| `cost_entries` | `id` PK, `driver_sub`, `org_id`/`hub_id` (V3), `category` enum CHECK, `amount_minor` BIGINT CHECK `0..10^12`, `currency` CHECK `^[A-Z]{3}$`, `note`, `day` | `amount_minor` = unit minor (sen/rp), fixed-point — **bukan** float |
| `vehicle_checks` | `id` PK, `driver_sub`, `org_id`, `hub_id` NOT NULL + RESTRICT (V3), `day`, `driver_name`, `license_plate`, `vehicle_type`, `km_start`/`km_end` BIGINT + `km_end >= km_start` CHECK, `condition` enum CHECK, `items`/`payload` JSONB, `service_date`/`kir_date`/`stnk_date` | Daftar tanggal STNK/KIR disimpan TEXT `YYYY-MM-DD` (tanggal kalender lokal, bukan instant) |
| `gps_observations` | `id` PK, `org_id`, `hub_id`/`driver_sub`/`plate` (SET NULL), `source`/`quality` enum CHECK, `lat`/`lng` range CHECK, `accuracy_meters`, `speed_mps`, `mock_reported`, `recorded_at`, `day` | Invariant kontrak (V3): `vehicle_gps ⇒ plate NOT NULL`; `quality='unavailable' ⇒ tanpa fix` |
| `gps_reviews` | `id` PK, `org_id`, `hub_id` NOT NULL + RESTRICT (V3), `driver_sub`/`plate` (SET NULL), `app_observation_id`/`vehicle_observation_id` (**bukan FK lagi** — V3), `classification`/`reason` enum CHECK, `separation_meters`, `time_delta_seconds`, `reviewed_by`, `created_at`, `day` | Observasi diprune 72 jam; review hidup lebih lama sebagai audit — FK ke `gps_observations` dijatuhkan agar prune tidak memutuskan referensi |

### 1.4 Alasan tipe yang tidak jelas

| Pilihan | Alasan |
|---|---|
| `TIMESTAMPTZ`, bukan `TIMESTAMP` | Semua waktu adalah instant UTC; zona waktu perangkat disimpan terpisah di `offset_min` (`DeviceTime`) — menyimpan `timestamp` naive akan menghilangkan sumber kebenaran zona |
| `day TEXT 'YYYY-MM-DD'`, bukan `DATE` | `day` adalah kunci kalender **di zona lokal hub/driver** (`DateOnlySchema`), bukan instant; mengubah ke `DATE` tidak menambah apa-apa karena tidak ada query range atasnya — dan TEXT menjaga kompatibilitas dengan apa yang dikirim perangkat. Formatnya dikunci CHECK `^\d{4}-\d{2}-\d{2}$` (V3) |
| `amount_minor BIGINT`, bukan `NUMERIC`/`float` | `MoneySchema` memakai unit minor integer; `BIGINT` menampung hingga 10^12 dengan presisi exact. Float dilarang untuk uang |
| `payload`/`items`/`visited_task_ids` JSONB | Bagian kontrak yang sengaja untyped (`DeviceEvent.payload`) atau nested array (`CheckItem[]`) — dinormalisasi tidak menambah invariant apa pun, hanya menambah join |
| `lat`/`lng` `DOUBLE PRECISION` | Koordinat bukan kuantitas exact; `float8` cukup dan match `f64` Rust. Range dikunci CHECK |
| `INTEGER → BIGINT` untuk `sequence`, `service_seconds`, `revision`, `odometer_*`, `km_*` (V3) | Rust mengikat `u32` sebagai `i64`; `int4` menolaknya **dan** tidak menampung rentang penuh u32 |

### 1.5 Entitas kontrak yang belum ada tabelnya (tidak dibuat — menunggu fitur)

`flows`, `route_plans`, `route_comparisons`, `check_in_out`, `entity_data`/`data_types`,
`currencies`, `custom_modules`, `page_webhook_queue`, `sync_failure_reports`.
Kontrak Zod-nya ada, tapi belum ada writer/reader di store — membuat tabel sekarang
adalah skema spekulatif. Saat fitur mendarat, tabel dibuat lewat migrasi baru dengan
aturan yang sama.

---

## 2. Strategi Kunci

| Tabel | PK | Alasan |
|---|---|---|
| Hampir semua | `TEXT` (UUIDv4 dari `new_id()`, atau id dari client seperti `event_id`) | Kontrak `IdSchema` mengizinkan id non-UUID (`obs:…`, id bentukan perangkat); kolom `uuid` akan menolak id tersebut. TEXT menjaga kontrak, bukan konvensi |
| `organizations`, `hubs`, `teams`, `users`, `vehicles` | `TEXT` natural/id dari domain (`plate`, `sub` Keycloak, `org_id` dari provisioning) | Id-nya dimiliki sistem lain (Keycloak, McEasy) atau natural key (`plate`); tidak perlu surrogate |
| Junction (`user_orgs`, `user_hubs`, `org_hubs`, `team_*`, `hub_vehicles`, `user_devices`, `task_assignments`) | PK komposit | Mencegah duplikat relasi secara gratis; juga berfungsi sebagai indeks lookup arah pertama |
| `daily_reports` | `(driver_sub, day)` → V4: `(org_id, driver_sub, day)` | Satu report per driver per hari per tenant |

**Sequential vs UUID vs ULID:** ID saat ini adalah UUIDv4 (acak → fragmentasi B-tree pada
`device_events`/`gps_observations`, dua tabel tulis-berat). Ini diterima karena:
(a) `event_id`/`observation id` ditetapkan client dan harus opaque;
(b) volume insert moderat (lihat §7);
(c) urutan waktu tetap tersedia via `occurred_utc`/`recorded_at`/`created_at` yang
terindeks — query tidak pernah `ORDER BY id` untuk urutan waktu.
Kalau `gps_observations` nanti melewati ~10 juta baris, evaluasi ULID/time-ordered id
untuk mengurangi fragmentasi — **dengan angka**, bukan spekulasi.

---

## 3. Constraint sebagai Invariant

INV-id diturunkan dari `superRefine`/validasi di `packages/api-contracts/src/*.ts`
(tidak ada dokumen `01 §3`; rujukan = file kontrak).

| INV-id | Invariant | Penegakan |
|---|---|---|
| INV-01 | Task stage ∈ 5 nilai kontrak (`TaskStatusSchema`) | `tasks_stage_check` (V3 — menambah `unassigned` yang V1 hilangkan) |
| INV-02 | Stop stage ∈ 6 nilai (`StopStatusSchema`) | `stops.stage` CHECK (V1) |
| INV-03 | Sequence stop unik per task (`TaskSchema.superRefine`) | `stops_task_sequence_unique` (V3) |
| INV-04 | Idempotensi event per (org, driver, request_key) | `UNIQUE(org_id, driver_sub, request_key)` + replay short-circuit di `record_event` |
| INV-05 | `offset_min` ∈ [-840, 840] (`DeviceTimeSchema`) | CHECK (V1) |
| INV-06 | `amount_minor` ∈ [0, 10^12] (`MoneySchema`) | `cost_entries_amount_minor_check` (V1) |
| INV-07 | `currency` = 3 huruf besar | `cost_entries_currency_check` (V1) |
| INV-08 | `category` ∈ 6 kategori (`ExpenseCategorySchema`) | CHECK (V3) |
| INV-09 | `day` = `YYYY-MM-DD` (`DateOnlySchema`) | CHECK regex di 7 tabel (V3) |
| INV-10 | `lat` ∈ [-90,90], `lng` ∈ [-180,180] (`CoordinateSchema`) | CHECK di `hubs`/`stops`/`gps_observations` (V3) |
| INV-11 | `vehicle_gps` ⇒ `vehicleId` wajib (`GpsObservationSchema`) | `gps_obs_vehicle_requires_plate` (V3) |
| INV-12 | `quality='unavailable'` ⇒ tanpa posisi/akurasi | `gps_obs_unavailable_has_no_fix` (V3) |
| INV-13 | Arah sebaliknya: fix ⇒ `position`+`accuracy` wajib | **TIDAK ditegakkan** — `record_event` menulis `accurate` dengan `accuracy_meters NULL`; perlu perbaikan kode dulu, lalu CHECK di V4 |
| INV-14 | `app_gps` + posisi ⇒ `mockLocationReported` wajib | **TIDAK ditegakkan** — `record_event` menulis `mock_reported NULL`; perbaikan kode dulu (V4) |
| INV-15 | LHS status ∈ 4 nilai (`LhsStatusSchema`) | `daily_reports_status_check` (V3) |
| INV-16 | LHS status/revision/reviews server-owned | Tidak bisa di DB — penegakan di route (`record_report` menimpa dari principal). **Alasan:** CHECK tidak bisa membedakan "ditulis server" vs "dikirim client" |
| INV-17 | Transisi review LHS berurutan (`LhsReportSchema.superRefine`) | **App-level** — DB hanya menyimpan status terkini; riwayat review penuh belum ada tabelnya (fitur review belum mendarat) |
| INV-18 | `km_end >= km_start` (`VehicleCheckSchema`) | `vehicle_checks_km_order` (V3) |
| INV-19 | `condition` ∈ `good`/`not_good` | `vehicle_checks_condition_check` (V3) |
| INV-20 | Duplikat item check `(category,name)` dilarang | **App-level** — `items` adalah JSONB; CHECK tidak bisa melihat ke dalam array. Alternatif normalisasi `check_items` dinilai tidak sebanding (tidak ada query per-item) |
| INV-21 | Assignee task ∈ anggota org task | `task_assignments_member_fk` komposit `(org_id, driver_sub)→user_orgs` (V3) — **menutup celah: dulu tidak dicek di mana pun** |
| INV-22 | Task's hub ∈ org task | `task_assignments_task_fk` komposit `(task_id, org_id)→tasks(id, org_id)` + app check `org_hubs` di `create_task`/`update_task` |
| INV-23 | Event hanya pada task/stop dalam org pemanggil | App-level di `record_event` (JOIN `org_hubs` + `FOR UPDATE`) — FK tidak bisa mengekspresikan "stop ini ∈ task ∈ org ini" tanpa kolom `org_id` pada `stops`; pertimbangkan di V4 bila jalur tulis bertambah |
| INV-24 | Hub tidak bisa dihapus bila masih dipakai task/report/check/review | `ON DELETE RESTRICT` (V3) + app `hub_in_use` (perlu diperluas — lihat §6) |
| INV-25 | Hapus org/users/devices menghapus keanggotaan | `ON DELETE CASCADE` pada semua junction |
| INV-26 | Event/review bertahan saat task/stop/user/vehicle dihapus | `ON DELETE SET NULL` pada `task_id`/`stop_id`/`driver_sub`/`plate` (audit tidak ikut hilang) |
| INV-27 | Transisi stop legal (`check_transition`) | **App-level** — state machine `pending→arrived→working→completed→departed` / `pending→skipped`; bisa jadi CHECK transisi hanya lewat trigger — dinilai tidak sebanding dengan kompleksitas |
| INV-28 | `record_event` atomic (event + stage + task rollup) | Transaksi tunggal di `PgStore::record_event` — bukan constraint, tapi batas transaksi (§5) |
| INV-29 | `source`/`quality`/`classification`/`reason`/`action` enum | CHECK di `gps_observations`, `gps_reviews`, `device_events` (V3) |
| INV-30 | `id` non-empty, shape `[A-Za-z0-9][A-Za-z0-9_.:-]*` ≤128 (`IdSchema`) | **App-level** — kontrak mengizinkan semua id valid; CHECK regex pada setiap kolom id dinilai noise (Zod sudah menolak di edge). Ditambahkan nanti bila ada writer non-API |
| INV-31 | `timeZone` IANA valid (`OrganizationSchema`/`HubSchema`) | Kolom ada (V3); validasi IANA app-level — CHECK tidak bisa memverifikasi nama zona |
| INV-32 | Membership `hubIds`/`permissions` unik (`MembershipSchema`) | `user_hubs` PK komposit mencegah hub duplikat; `permissions` belum ada tabelnya |
| INV-33 | Task assigned ⇒ driver ada; unassigned ⇒ tanpa driver (`TaskSchema.superRefine`) | **App-level** — `assignee_id` hidup di `task_assignments` (relasi), bukan kolom task; CHECK tidak bisa lintas tabel. `task_assignments_member_fk` menutup separuh kasus (assignee harus member) |

---

## 4. Indeks

Semua indeks turunan dari query di `PgStore` (63 statement, semuanya parameterized —
tidak ada SQL dibangun via `format!`).

| Indeks | Kolom | Query yang membenarkan | Selektivitas |
|---|---|---|---|
| `idx_tasks_org_id` (V3, ganti `idx_tasks_org`) | `(org_id, id)` | `tasks_for_org`: `WHERE org_id=$1 ORDER BY id` — filter+urut satu index | tinggi (per-org) |
| `tasks_hub_id` index `idx_tasks_hub` | `(hub_id)` | `hub_in_use` EXISTS + FK index utk RESTRICT | tinggi |
| `stops_task_sequence_unique` (V3) | `(task_id, sequence)` UNIQUE | agregasi `WHERE task_id ORDER BY sequence` di setiap baca task; juga melayani resolusi stop non-terminal `record_event` | 1 stop per baris |
| `device_events` UNIQUE | `(org_id, driver_sub, request_key)` | idempotency check `record_event` | exact-match |
| `idx_device_events_org` | `(org_id)` | FK index (`org_id→organizations`) + scoping | tinggi |
| `idx_device_events_driver` | `(driver_sub, day)` | FK index + riwayat per driver per hari | tinggi |
| `idx_device_events_task` | `(task_id)` | FK index (`task_id→tasks` SET NULL) | tinggi |
| `idx_device_events_stop` (V3) | `(stop_id)` | FK index (`stop_id→stops` SET NULL) | tinggi |
| `idx_daily_reports_hub` (V3) | `(hub_id)` | FK index (RESTRICT) | tinggi |
| `idx_cost_entries_driver` | `(driver_sub, day)` | `costs_for_driver`: `WHERE driver_sub=$1 AND day=$2` | tinggi |
| `idx_vehicle_checks_org` | `(org_id, day)` | `vehicle_checks_for_org`: `WHERE org_id=$1 [AND hub_id]` `ORDER BY day` | tinggi |
| `idx_vehicle_checks_hub`/`_driver` (V3) | `(hub_id)`, `(driver_sub)` | FK indexes (RESTRICT/CASCADE) | tinggi |
| `idx_gps_observations_org` | `(org_id, recorded_at)` | `gps_observations_for_org`: `WHERE org_id AND recorded_at>=` `ORDER BY recorded_at` | tinggi |
| `idx_gps_observations_driver` | `(driver_sub, day)` | riwayat per driver + FK index | tinggi |
| `idx_gps_observations_hub`/`_plate` (V3) | `(hub_id)`, `(plate)` | FK indexes (SET NULL) — tanpa ini, hapus hub/kendaraan seq-scan tabel terpanas | tinggi |
| `idx_gps_observations_recorded` (V3) | `(recorded_at)` | `prune_gps_observations_older_than`: `DELETE WHERE recorded_at < $1` **lintas-org** — indeks `(org_id, recorded_at)` tidak melayaninya | range-delete |
| `idx_gps_reviews_org` | `(org_id, created_at)` | `gps_reviews_for_org`: `WHERE org_id ORDER BY created_at` | tinggi |
| `idx_gps_reviews_driver` | `(driver_sub, day)` | riwayat + FK index | tinggi |
| `idx_gps_reviews_hub`/`_plate` (V3) | `(hub_id)`, `(plate)` | FK indexes (RESTRICT/SET NULL) | tinggi |
| `user_orgs_org_user_unique` (V3) | `(org_id, user_sub)` UNIQUE | `user_in_org` `WHERE org_id AND user_sub`; `users_for_org`/`drivers_for_org` `WHERE uo.org_id=$1`; **target FK** `task_assignments` | exact-match |
| `user_orgs` PK | `(user_sub, org_id)` | `organization_of`/`organization_and_hub_of` `WHERE user_sub=$1` | exact-match |
| `idx_user_hubs_hub` (V3) | `(hub_id)` | FK index + `hub_in_use` | tinggi |
| `user_hubs` PK | `(user_sub, hub_id)` | `organization_and_hub_of` join | exact-match |
| `idx_org_hubs_hub` (V3) | `(hub_id, org_id)` | join `org_hubs` dipimpin `hub_id` di `record_event`, `update_task`, `teams_for_org`, `mceasy_sync_vehicle` | exact-match |
| `idx_team_hubs_hub` | `(hub_id)` | `hub_in_use` + join teams_for_org | tinggi |
| `idx_team_members_user` (V3) | `(user_sub)` | FK index (CASCADE saat user dihapus) | tinggi |
| `idx_task_assignments_member` (V3) | `(org_id, driver_sub)` | FK index untuk cascade dari `user_orgs` | exact-match |
| `idx_hub_vehicles_plate` (V3, ganti `idx_hub_vehicles_hub`) | `(plate, hub_id)` | `mceasy_sync_vehicle`: `WHERE hv.plate=$2` | exact-match |
| `idx_user_devices_device` (V3, ganti `idx_user_devices_user`) | `(device_id)` | `register_push_token`: `WHERE device_id=$1` | exact-match |

**Dihapus di V3 (redundant — prefix PK/UNIQUE sudah menutup):**
`idx_user_orgs_user`, `idx_user_hubs_user`, `idx_org_hubs_org`,
`idx_hub_vehicles_hub`, `idx_user_devices_user`, `idx_daily_reports_driver`,
`idx_stops_task`, `idx_tasks_org`.

**Sengaja TIDAK dibuat:**
- `task_assignments(driver_sub)` — tidak ada query "tugas per driver" di `PgStore` saat ini (`list_tasks` belum memfilter assignee). Tambahkan bersama querynya.
- `devices.fcm_token` UNIQUE — token FCM dapat berpindah antar device_id (reinstall); unik akan memutuskan rotasi yang sah.
- Partial index `stops` non-terminal — volume stops per task kecil (≤500 kontrak, tipikal <20); `(task_id, sequence)` cukup.
- `payload` JSONB GIN — tidak ada query atas isi payload.

---

## 5. Batas Transaksi

| Operasi | Atomik atas | Isolation |
|---|---|---|
| `record_event` | idempotency check → resolusi stop (`FOR UPDATE`) → insert event → update stage stop → rollup stage task → insert observasi GPS app | READ COMMITTED + `FOR UPDATE` pada stop |
| `create_task` | cek `org_hubs` → insert task → insert stops → insert assignment | READ COMMITTED |
| `update_task` | cek `org_hubs` → update task → update stop → hapus+insert assignment | READ COMMITTED |
| `provision_user`, `create_hub`, `create_team`, `register_push_token` | upsert/link multi-tabel | READ COMMITTED |
| `delete_team`, `delete_hub` | multi-DELETE berurutan | READ COMMITTED |

READ COMMITTED cukup: semua race condition konkret diselesaikan lewat UNIQUE +
`FOR UPDATE`, bukan lewat snapshot — SERIALIZABLE tidak menambah apa-apa di sini.

**Risiko deadlock yang ditemukan (perlu perbaikan kode):**

`record_event` mengunci **stop dulu** (`SELECT ... FOR UPDATE OF s`), lalu meng-update
**task** (`UPDATE tasks SET stage=…`). `update_task` mengunci **task dulu**
(`UPDATE tasks`), lalu meng-update **stop**. Urutan lock berlawanan → deadlock
klasik ketika event driver dan update task staff tiba bersamaan pada task yang sama.

```
T1 (record_event):  lock stop S …        want task T  ─┐
T2 (update_task):   lock task T …        want stop S  ─┘  DEADLOCK
```

Perbaikan (direkomendasikan, belum diterapkan): `record_event` mengunci baris
`tasks` lebih dulu (`SELECT 1 FROM tasks WHERE id=$2 AND org_id=$1 FOR UPDATE`),
baru kemudian `stops` — menyamakan urutan lock menjadi `tasks → stops` di kedua
transaksi. Deteksi di produksi: `log_lock_waits=on`, `deadlock_timeout=1s`,
metrik `pg_stat_database.deadlocks`.

**Isu uji yang ditemukan:** `Fixture::new` di `pg_it.rs` menelan kegagalan
`migrate()` (`.ok()?` → `None` → test return cepat → dihitung **lulus**). Pada
run paralel, dua fixture yang balapan migrasi bisa membuat satu test "lulus" tanpa
pernah menyentuh DB. Rekomendasi: serialize migrate lewat satu `tokio::sync::Mutex`
statik, dan `expect()` — bukan `ok()?` — agar kegagalan migrasi mematikan test.

---

## 6. Rencana Migrasi

Refinery menjalankan tiap file `V*__*.sql` di dalam **satu transaksi** (atomic per
migrasi, rollback otomatis bila gagal). Konsekuensi: `CREATE INDEX CONCURRENTLY`
dilarang di dalam file migrasi — pada volume saat ini tidak masalah; ketika tabel
sudah besar, build indeks dilakukan manual di luar refinery lalu dicatat.

`migrate()` dipanggil sekali saat startup API. Untuk deploy multi-instance: migrasi
harus di-serialize (lock advisory `pg_advisory_lock` di awal `migrate()`, atau job
migrasi terpisah di pipeline) — dua instance yang start bersamaan bisa balapan
`refinery_schema_history`.

### V3__contract_integrity (tervalidasi — diterapkan pada scratch DB, 7/7 pg_it lulus)

Fase **expand** — kompatibel mundur terhadap kode saat ini.

| Perubahan | Mengunci tabel? | Estimasi |
|---|---|---|
| CHECK enum/format/range (A–D) | `ACCESS EXCLUSIVE` sebentar, validasi scan tabel kecil | <1s pada volume saat ini; gunakan `NOT VALID`+`VALIDATE CONSTRAINT` bila tabel besar |
| `UNIQUE(task_id, sequence)` (E) | Lock pendek, build index kecil | <1s |
| Kolom baru nullable (F, I) | `ACCESS EXCLUSIVE` instan (metadata-only, `DEFAULT` konstan di PG ≥11 tidak rewrite) | instan |
| Backfill `org_id`/`hub_id` (F, G) | `UPDATE` row-lock per baris; tabel kecil | <1s |
| FK komposit + `UNIQUE(id, org_id)`/`(org_id, user_sub)` (G) | Lock pendek; build 2 index kecil | <1s |
| FK `RESTRICT` + `NOT NULL` (H, I) | Lock pendek, validasi scan | <1s |
| `INTEGER→BIGINT` (J) | **Table rewrite** pada `stops`/`daily_reports`/`vehicle_checks` | instan pada volume saat ini; pada volume besar lakukan off-peak |
| Indeks baru + drop redundant (K) | `CREATE INDEX` share-lock pendek | <1s |

**Pre-flight sebelum V3 pada DB berisi data:**

```sql
-- Duplikat sequence yang akan menggagalkan UNIQUE:
SELECT task_id, sequence, count(*) FROM stops
  GROUP BY 1,2 HAVING count(*) > 1;
-- Assignment yang melanggar FK komposit (assignee bukan anggota org):
SELECT ta.task_id, ta.driver_sub FROM task_assignments ta
  JOIN tasks t ON t.id = ta.task_id
  WHERE NOT EXISTS (SELECT 1 FROM user_orgs uo
                    WHERE uo.org_id = t.org_id AND uo.user_sub = ta.driver_sub);
-- Baris audit dengan hub NULL (akan gagal SET NOT NULL):
SELECT count(*) FROM daily_reports WHERE hub_id IS NULL;
SELECT count(*) FROM vehicle_checks WHERE hub_id IS NULL;
SELECT count(*) FROM gps_reviews WHERE hub_id IS NULL;
```

**Skrip turun (V3):**

```sql
ALTER TABLE task_assignments
    DROP CONSTRAINT task_assignments_task_fk,
    DROP CONSTRAINT task_assignments_member_fk,
    ADD CONSTRAINT task_assignments_task_id_fkey
        FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    ADD CONSTRAINT task_assignments_driver_sub_fkey
        FOREIGN KEY (driver_sub) REFERENCES users(sub) ON DELETE CASCADE,
    DROP COLUMN org_id;
ALTER TABLE tasks DROP CONSTRAINT tasks_id_org_unique,
    DROP CONSTRAINT tasks_hub_id_fkey,
    ADD CONSTRAINT tasks_hub_id_fkey FOREIGN KEY (hub_id) REFERENCES hubs(id) ON DELETE CASCADE,
    DROP CONSTRAINT tasks_stage_check,
    ADD CONSTRAINT tasks_stage_check CHECK (stage IN ('assigned','in_progress','completed','cancelled'));
ALTER TABLE user_orgs DROP CONSTRAINT user_orgs_org_user_unique;
ALTER TABLE stops DROP CONSTRAINT stops_task_sequence_unique,
    ALTER COLUMN sequence TYPE INTEGER, ALTER COLUMN service_seconds TYPE INTEGER;
ALTER TABLE daily_reports ALTER COLUMN hub_id DROP NOT NULL,
    DROP CONSTRAINT daily_reports_hub_id_fkey,
    ADD CONSTRAINT daily_reports_hub_id_fkey FOREIGN KEY (hub_id) REFERENCES hubs(id) ON DELETE SET NULL,
    ALTER COLUMN revision TYPE INTEGER, ALTER COLUMN odometer_start TYPE INTEGER,
    ALTER COLUMN odometer_end TYPE INTEGER, DROP COLUMN org_id;
ALTER TABLE vehicle_checks ALTER COLUMN hub_id DROP NOT NULL,
    DROP CONSTRAINT vehicle_checks_hub_id_fkey,
    ADD CONSTRAINT vehicle_checks_hub_id_fkey FOREIGN KEY (hub_id) REFERENCES hubs(id) ON DELETE SET NULL,
    ALTER COLUMN km_start TYPE INTEGER, ALTER COLUMN km_end TYPE INTEGER;
ALTER TABLE gps_reviews ALTER COLUMN hub_id DROP NOT NULL,
    DROP CONSTRAINT gps_reviews_hub_id_fkey,
    ADD CONSTRAINT gps_reviews_hub_id_fkey FOREIGN KEY (hub_id) REFERENCES hubs(id) ON DELETE SET NULL,
    ADD CONSTRAINT gps_reviews_app_observation_id_fkey FOREIGN KEY (app_observation_id) REFERENCES gps_observations(id) ON DELETE SET NULL,
    ADD CONSTRAINT gps_reviews_vehicle_observation_id_fkey FOREIGN KEY (vehicle_observation_id) REFERENCES gps_observations(id) ON DELETE SET NULL;
ALTER TABLE device_events ALTER COLUMN device_id DROP NOT NULL, DROP COLUMN hub_id;
ALTER TABLE cost_entries DROP COLUMN org_id, DROP COLUMN hub_id;
ALTER TABLE organizations DROP COLUMN time_zone;
ALTER TABLE hubs DROP COLUMN time_zone;
ALTER TABLE users DROP COLUMN email;
-- + DROP semua CHECK (A–D) dan index baru (K); CREATE ulang index lama yang di-drop.
```

### V4 — fase CONTRACT (`V4__contract_phase.sql`)

**Pola expand→backfill→contract:** SET NOT NULL / PK / CHECK dijalankan **hanya
setelah** deploy Phase 0 yang dual-write kolom tenant/GPS/email + backfill di
file migrasi. Jangan terapkan V4 terhadap DB yang masih ditulis oleh biner lama.

Path: `backend/crates/altius-api/migrations/V4__contract_phase.sql`

| Perubahan | Prasyarat kode / backfill |
|---|---|
| Kolom kontrak `device_events`: `schema_version`, `device_sequence`, `expected_task_revision`, `reason`, `observation_id` (nullable) | `DeviceEvent` + `record_event` INSERT bindings |
| CHECK `device_events_skip_requires_reason` | `sync_events` menolak `skip` tanpa reason non-kosong |
| `task_assignments.org_id SET NOT NULL` | INSERT di `create_task`/`update_task` + backfill dari `tasks.org_id` |
| `device_events.hub_id SET NOT NULL` | `record_event` menulis `ev.hub_id`; backfill via `tasks` lalu `org_hubs` |
| `cost_entries.org_id`/`hub_id` `SET NOT NULL` | `record_cost` scope principal; backfill `user_orgs`/`user_hubs`/`org_hubs` |
| `daily_reports.org_id SET NOT NULL`; PK → `(org_id, driver_sub, day)` | `record_daily_report` membawa org; backfill `user_orgs` |
| `users.email SET NOT NULL` | `provision_user(..., email)`; backfill `sub \|\| '@users.invalid'` |
| `gps_observations` CHECK accurate⇒fix+accuracy; app_gps+fix⇒`mock_reported` (INV-13/14) | `record_event` hanya menulis observasi bila `accuracy` ada; `mock_reported` dari payload |
| Index `idx_device_events_occurred` | Mendukung prune retensi (worker terpisah) |

**Pre-flight sebelum V4 pada DB berisi data:**

```sql
-- Baris yang akan gagal SET NOT NULL setelah backfill (harusnya 0 setelah UPDATE):
SELECT count(*) FROM task_assignments WHERE org_id IS NULL;
SELECT count(*) FROM device_events WHERE hub_id IS NULL;
SELECT count(*) FROM cost_entries WHERE org_id IS NULL OR hub_id IS NULL;
SELECT count(*) FROM daily_reports WHERE org_id IS NULL;
SELECT count(*) FROM users WHERE email IS NULL OR btrim(email) = '';
-- GPS yang akan gagal CHECK INV-13/14:
SELECT count(*) FROM gps_observations
  WHERE quality = 'accurate'
    AND (lat IS NULL OR lng IS NULL OR accuracy_meters IS NULL);
SELECT count(*) FROM gps_observations
  WHERE source = 'app_gps' AND lat IS NOT NULL AND mock_reported IS NULL;
```

**Skrip turun (V4) — ringkas:** drop CHECK GPS + skip-reason; restore
`daily_reports` PK `(driver_sub, day)`; `ALTER … DROP NOT NULL` pada kolom
kontrak; `DROP COLUMN` lima kolom kontrak `device_events`; drop
`idx_device_events_occurred`.

Pola expand–contract untuk perubahan tak-kompatibel ke depan (contoh: rename kolom):
`ADD kolom_baru` → dual-write di kode → backfill → pindahkan pembaca →
`DROP kolom_lama` di migrasi berikutnya. Jangan pernah `RENAME`/tipe-baru
dalam satu langkah pada kolom yang sedang ditulis.

---

## 7. Volume & Retensi

Estimasi basis: 100 driver aktif per org besar, hub operasional 12 jam.

| Tabel | Pertumbuhan | Retensi | Risiko pertama |
|---|---|---|---|
| `gps_observations` | App GPS per event + telemetri kendaraan tiap `MCEASY_POLL_SECONDS=60` → ~1.440/hari/kendaraan → **~150–300rb baris/hari/org** | **72 jam** via `prune_gps_observations_older_than` (worker McEasy) — `idx_gps_observations_recorded` (V3) melayani DELETE lintas-org | Tabel terpanas. Prune harus dijadwalkan berkala; evaluasi `PARTITION BY RANGE (recorded_at)` bila >10 jt baris live |
| `device_events` | ~50–500/hari/driver → **~5–50rb/hari/org** | **`EVENTS_RETENTION_DAYS` (default 90)** via `prune_device_events_older_than` — periodic worker in `main.rs` (independent of McEasy); `idx_device_events_occurred` (V4) | Set `EVENTS_RETENTION_DAYS=0` to disable |
| `gps_reviews` | Sebagian kecil observasi | Tanpa retensi — audit | Kecil |
| `daily_reports` | 1/hari/driver → ~100/hari/org | Permanen (audit keuangan) | Kecil |
| `cost_entries` | ~5–20/hari/driver | Permanen (keuangan) | Kecil |
| `vehicle_checks` | 1–2/hari/driver | Permanen | Kecil |
| `tasks`/`stops` | ~puluhan–ratusan/hari/org | Permanen | Kecil |
| Master (`users`,`orgs`,`hubs`,`teams`,`vehicles`,`devices`) | Lambat | Permanen | Tidak relevan |

Urutan masalah: `gps_observations` (prune McEasy) → `device_events`
(`EVENTS_RETENTION_DAYS` / worker di `main.rs`) → sisanya tidak akan menjadi
masalah dalam waktu dekat.

---

## 8. Backup & Restore

| Komponen | Kebijakan |
|---|---|
| Full dump | `pg_dump -Fc` harian, disimpan ke object storage off-host (S3/GCS), retensi 30 hari |
| WAL archive | `archive_mode=on` + WAL-G/`wal_archive` → PITR; **RPO ≤ 15 menit** |
| Managed alternative | Bila memakai managed Postgres (RDS/Cloud SQL/dll.): snapshot harian + WAL → PITR bawaan; tetap lakukan restore drill |
| Restore drill | **Bulanan, wajib diuji** — backup yang belum pernah di-restore bukan backup |

Prosedur restore yang bisa diuji (lokal/staging):

```bash
# 1. PITR ke titik waktu via WAL archive, atau restore dump terakhir:
createdb altius_restore
pg_restore -d altius_restore --no-owner --no-acl backup_YYYYMMDD.dump

# 2. Verifikasi: migrasi konsisten + data inti ada
psql altius_restore -c "SELECT version, name FROM refinery_schema_history ORDER BY version;"
psql altius_restore -c "SELECT count(*) FROM organizations; SELECT count(*) FROM device_events;"

# 3. Arahkan instance API staging ke altius_restore (DATABASE_URL),
#    GET /api/v3/ready harus 200; jalankan pg_it smoke.
TEST_DATABASE_URL=postgres://…/altius_restore cargo test -p altius-api pg_it
```

**Target:** RPO ≤ 15 menit (WAL), RTO ≤ 1 jam (restore dump terakhir + replay WAL
ke titik insiden). Data yang sengaja hilang oleh retensi (`gps_observations` >72 jam)
tidak dipulihkan — itu desain, bukan kehilangan.

---

## 9. DDL

Skrip lengkap ada di `backend/crates/altius-api/migrations/`:
- `V1__init.sql` — skema inti (tenant, task, event, report, cost, check, device)
- `V2__gps.sql` — `gps_observations`, `gps_reviews`
- `V3__contract_integrity.sql` — fase expand: invariant kontrak, kunci tenant,
  FK komposit, perilaku delete, widening int4→bigint, indeks
- `V4__contract_phase.sql` — fase contract: SET NOT NULL tenant cols, PK
  `daily_reports`, kolom kontrak `device_events`, GPS CHECK INV-13/14,
  skip⇒reason CHECK, `users.email NOT NULL`

---

## Pemeriksaan Mandiri

- [x] Setiap invariant kontrak punya baris di §3 (INV-01..33); yang app-level disebut eksplisit beserta alasannya (INV-13/14/16/17/20/23/27/30/31/33)
- [x] Setiap indeks punya query atau operasi FK yang membenarkannya (§4); yang redundant di-drop; yang "jaga-jaga" tidak dibuat
- [x] Kolom nullable yang kontraknya wajib ditandai & diperbaiki sepanjang kompatibel dengan kode sekarang (`hub_id` audit → NOT NULL; `org_id` baru → V4)
- [x] Semua FK punya on-delete yang disengaja: CASCADE (junction/child), SET NULL (audit bertahan), RESTRICT (audit memblokir penghapusan hub) — keputusan non-default didokumentasikan
- [x] V3 tervalidasi pada scratch DB; 7/7 `pg_it` lulus; smokescript pass/fail semua berperilaku benar
- [x] Skrip turun V3 ada (§6)
- [x] Query terpanas (`tasks_for_org`, `record_event`, `costs_for_driver`, `gps_observations_for_org`, prune) punya indeks yang melayani filter **dan** urutan
- [x] Uang = `BIGINT` minor units, bukan float
- [x] Semua instant = `TIMESTAMPTZ`; `day` = teks kalender lokal dengan CHECK format (keputusan disengaja, §1.4)

**Ditemukan saat penulisan (di luar perbaikan skema):**
1. `pg_it::Fixture::new` menelan kegagalan `migrate()` (`.ok()?` → `None` → test "lulus" tanpa DB). Diperbaiki: migrate diserialkan lewat `Mutex` statik dan `expect()` — kegagalan migrasi kini membunuh test.
2. `link_admin_user` tidak menulis `user_hubs` → `organization_and_hub_of` admin selalu `None`. Diperbaiki: admin bootstrap kini ditautkan ke hub default.
3. `tasks_for_org`/`task_by_id` memakai `row_to_json(t) || jsonb` — `row_to_json` mengembalikan `json` (bukan `jsonb`), `||` gagal. Diperbaiki: `to_jsonb(t)`.
4. `push_tokens_and_mceasy_sync` memakai device id hardcoded `"dev-1"` → kolisi antar test paralel pada DB bersama. Diperbaiki: `uniq("dev")`.
5. Bug laten `i64→int4` pada 7 binding (`sequence`, `service_seconds`, `revision`, `odometer_*`, `km_*`) — kolom diperlebar ke `BIGINT` (V3 §J); `int4` juga tidak menampung rentang penuh `u32`.
6. `record_event` vs `update_task` punya urutan lock berlawanan → risiko deadlock (§5, perlu perbaikan kode).
7. `record_report` memakai `unwrap_or_default()` untuk hub → driver tanpa hub menghasilkan `hub_id=''` → 500 di FK (§6 V4).
8. `device_events` tidak punya retensi — tumbuh tak terbatas (§7).
