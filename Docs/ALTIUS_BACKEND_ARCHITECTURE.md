# Altius Backend Architecture

Target backend for the Altius Field Task Management platform. This document
defines the service topology, data model, and integration contracts. The
reference implementation lives in [`backend/`](../backend/).

## Topology

| Component | Technology | Responsibility |
|---|---|---|
| API service | Rust, Axum | REST/JSON endpoints, auth enforcement, orchestration |
| Identity | Keycloak | OIDC provider: realm per environment, clients for web/mobile, roles map to admin/supervisor/lead/driver |
| Transactional store | PostgreSQL | Source of truth for orgs, hubs, users, tasks, events, reports, costs, checks, GPS reviews — ACID, constraints, `tokio-postgres` + `deadpool` |
| Graph store | TypeDB 3.x | Optional relation/inference backend (`STORE_BACKEND=typedb`) for n-ary relations and evolving schemas |
| Local analytics | DuckDB | Planned: file-based exports and offline analysis |
| Real-time analytics | ClickHouse | Planned: dashboard aggregations over events/GPS/costs |
| Routing | Google Maps Platform | Directions + Distance Matrix + Geocoding for real road ETA |
| Agent service | OpenRouter (go-agent semantics) | LLM tool loop: dispatch suggestions, anomaly summarization, report drafting |
| Object store | S3-compatible | Task media (photo/voice/proof drafts) — presigned URLs only |

The current `apps/web` and `apps/mobile` clients remain fully functional
against local mock adapters; this backend is the production integration gate.

## Authentication (Keycloak)

- Realm `altius` per environment (dev/stage/prod), following the
  `https://github.com/keycloak/keycloak` deployment model.
- Clients: `altius-web` (public, PKCE), `altius-mobile` (public, PKCE +
  refresh tokens), `altius-agent` (confidential, service account).
- Realm roles map to the workspace personas: `admin`, `supervisor`, `lead`,
  `driver`. Tenant/org/hub scoping is enforced server-side by resolving the
  JWT `sub` against the TypeDB membership graph — never by trusting claims
  alone.
- The Rust API validates `Authorization: Bearer` tokens against the realm
  JWKS (RS256), checking `iss`, `aud`, `exp`, and signature. JWKS is cached
  with a short TTL and refreshed on unknown `kid`.

## Data model (PostgreSQL)

PostgreSQL is the system of record. The schema lives in
`backend/crates/altius-api/migrations/` and mirrors the Zod contracts in
`packages/api-contracts`:

- **Tables**: `organizations`, `hubs`, `users`, `teams`, `tasks`, `stops`,
  `device_events`, `daily_reports`, `cost_entries`, `vehicle_checks`,
  `vehicles`, `devices`, `gps_observations`, `gps_reviews`.
- **Join tables**: `user_orgs`, `user_hubs`, `org_hubs`, `team_hubs`,
  `team_members`, `task_assignments`, `hub_vehicles`, `user_devices`.
- `device_events.request_key` is `UNIQUE (org_id, driver_sub, request_key)`,
  matching the mobile `events.request_key` unique index.
- Multi-statement writes (`create_task`, `record_event`) run inside explicit
  transactions, preserving the atomicity the mobile `WorkStore` guarantees
  locally.

TypeDB remains in `backend/crates/altius-schema` and
`backend/crates/altius-api/src/store/typedb.rs` for future relation/inference
workloads; it is not used by the default `STORE_BACKEND=postgres` path.

## Google Maps integration

- **Directions API**: per-task stop order → polyline + leg durations for
  planned ETA. Called server-side only; keys never ship to clients.
- **Distance Matrix API**: candidate stop-pair durations feed the Dijkstra /
  sequencing logic in `packages/algos` (ported to Rust in
  `crates/altius-routing`).
- **Geocoding API**: address → coordinate at task-create time; result cached
  on the `stop` entity.
- Graceful degradation: without `GOOGLE_MAPS_API_KEY` the service falls back
  to the demo schematic graph and marks responses `"maps": "demo"`.

## Agent service (OpenRouter / go-agent semantics)

The agent layer follows the `OpenRouterTeam/go-agent` contract — tool
execution loop, approval gates, stop conditions, serialized conversation
state — reimplemented in Rust (`crates/altius-agent`) so the backend stays
single-runtime. If a Go sidecar is preferred, `go-agent` can be vendored
directly with identical semantics.

Tools exposed to the model:

| Tool | Backed by |
|---|---|
| `plan_route` | Google Maps Distance Matrix + routing solver |
| `flag_anomaly` | GPS stream comparison (`compareGpsStreams` port) |
| `summarize_day` | TypeDB aggregation over events/costs |
| `draft_report` | LHS assembly for supervisor review |

Guardrails: HITL gate on `plan_route` and `draft_report` (a human approves
before anything is written); stop conditions `StepCountIs(8)` and
`MaxCost`; conversation state serialized per `ConversationStateVersion` so
runs resume across restarts.

## Endpoint surface

The API keeps the `/api/v3` shape documented in
[ALTIUS_API_REFERENCE.md](./ALTIUS_API_REFERENCE.md): auth delegated to
Keycloak (`POST /auth` becomes an OIDC code exchange), then tasks, events
sync, flows, LHS reports, and anomaly review under Bearer auth. Typed request
/response shapes are generated from `packages/api-contracts` Zod schemas —
Rust types in `altius-core` mirror them 1:1.

## Security posture

- No credentials in the repo; all secrets via env (`KEYCLOAK_*`,
  `DATABASE_URL`, `GOOGLE_MAPS_API_KEY`, `OPENROUTER_API_KEY`; `TYPEDB_*`
  only when `STORE_BACKEND=typedb`).
- Tenant isolation is a query-time guarantee: every SQL statement binds
  `org_id`/`driver_sub` resolved from the authenticated principal, never
  from client input.
- Events are immutable (API refuses updates; `device_events` is append-only),
  matching the mobile `events_no_update`/`events_no_delete` triggers.
- Rate limiting and body-size caps at the Axum layer; media uploads are
  presigned S3 URLs, never proxied.
