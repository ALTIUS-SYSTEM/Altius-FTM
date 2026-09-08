# Altius Backend Architecture

Production backend for **Altius FTM** (Fleet & Transport Management).  
Reference implementation: [`backend/`](../backend/).  
Last updated: 2026-09-08.

## Topology

| Component | Technology | Responsibility |
|---|---|---|
| API service | Rust, Axum | REST `/api/v3`, authz, orchestration |
| Identity | Keycloak | OIDC realm `altius`; clients `altius-web`, `altius-mobile` (PKCE), optional confidential agent client |
| Transactional store | PostgreSQL | System of record (`STORE_BACKEND=postgres`, default) |
| Graph store | TypeDB 3.x | Optional (`STORE_BACKEND=typedb`) |
| Routing | Google Maps Platform | Directions, Distance Matrix, Geocoding, static maps (server-side keys only) |
| Agent | OpenRouter tool loop | Dispatch suggestions with HITL gates |
| Object store | S3-compatible | Media via presigned URLs (planned wiring) |
| Edge | Caddy on VPS | TLS (ACME), rate limit, reverse proxy |

**Deploy:** backend on VPS (Compose + Caddy); `apps/web` / `apps/landing` on Vercel; mobile against the public API issuer.

Clients use the live API when Keycloak + `API_BASE` / `NEXT_PUBLIC_API_BASE` are configured. Local demo paths remain for offline UI work without IdP.

## Authentication (Keycloak)

- Realm `altius` per environment.
- Roles: `super-admin`, `admin`, `supervisor`, `lead`, `driver`.
- Tenant scoping: resolve JWT `sub` against Postgres membership (`user_orgs` / `user_hubs` / `org_hubs`) — never trust body-supplied `org_id` alone.
- API validates Bearer tokens (RS256 JWKS): `iss`, `aud`, `exp`, `nbf`, signature; JWKS cached with TTL + unknown-`kid` cooldown.
- Web: authorization code + PKCE; access token held in memory. Mobile: `flutter_appauth` + `FlutterSecureStorage`.

## Data model (PostgreSQL)

Schema: `backend/crates/altius-api/migrations/` (refinery). Design notes: [ALTIUS_DATABASE_DESIGN.md](./ALTIUS_DATABASE_DESIGN.md).

- Core tables: organizations, hubs, users, teams, tasks, stops, device_events, daily_reports, costs, vehicle_checks, vehicles, devices, gps_observations, gps_reviews (+ join tables).
- Idempotency: `UNIQUE (org_id, driver_sub, request_key)` on `device_events`.
- Event ingest advances stop/task stage; null `stop_id` resolves to the first active stop.
- TypeDB path remains available for inference experiments; not the default.

## Sync contract (mobile ↔ API)

1. Staff create/update tasks → `POST /api/v3/task-create`, `PUT /api/v3/task/{id}` (org from JWT).
2. Driver `GET /api/v3/tasks` → local SQLite (includes `stop_id`).
3. Outbox events → `POST /api/v3/events` (Bearer driver).
4. Receipts carry `event_id` / `server_event_id` (`accepted` | `conflict` | `rejected`); match by id, never array index.
5. Client pulls tasks again after sync.

## Maps & agent

- Maps keys stay on the server; missing key → schematic/`"maps": "demo"` responses.
- Agent tools: `plan_route`, `flag_anomaly`, `summarize_day`, `draft_report` with HITL on gated writes; conversation state server-owned on resume.

## Endpoint surface (implemented)

| Area | Routes |
|---|---|
| Liveness | `GET /api/v3/health`, `GET /api/v3/ready` |
| Auth | `POST /api/v3/auth/login`, `POST /api/v3/auth/refresh`, `GET /api/v3/auth/me` |
| Tasks / sync | `GET /api/v3/tasks`, `GET\|PUT /api/v3/task/{id}`, `POST /api/v3/task-create`, `POST /api/v3/events`, `POST /api/v3/route/optimize` |
| Maps | `POST /api/v3/route/eta\|geocode\|static-map`, `POST /api/v3/places/autocomplete` |
| Roster | users, drivers, hubs, teams, organization |
| LHS / costs | reports, costs, vehicle-checks |
| Monitoring | vehicles, gps reviews, McEasy sync hooks |
| Notify | sms / whatsapp / push (gated by provider config) |
| Agent | dispatch-suggestion, resume |

Broader historical/planned shapes: [ALTIUS_API_REFERENCE.md](./ALTIUS_API_REFERENCE.md).

## Security posture

- Secrets via env only (`KEYCLOAK_*`, `DATABASE_URL`, maps/agent keys; `TYPEDB_*` if typedb).
- Postgres TLS: `sslmode=require` → rustls + Mozilla roots; compose may use `sslmode=disable` on a private network.
- CORS allowlist (`CORS_ORIGINS`) for Vercel origins; empty denies browser cross-origin.
- Append-only events; 2 MiB body limit; security headers on responses.
- Rate limiting at Caddy/edge; see [SECURITY.md](../SECURITY.md).
