# Altius FTM — API Guide (integrators)

> **Audience:** Developers wiring `apps/web`, `apps/mobile`, or external systems to the VPS API.  
> **Product:** Altius FTM (Fleet & Transport Management)  
> **Date:** 2026-09-08  

## What to read

| Document | Use when |
|---|---|
| [openapi/altius-ftm-v3.openapi.yaml](./openapi/altius-ftm-v3.openapi.yaml) | Machine-readable **OpenAPI 3.0.3** for the **implemented** `/api/v3` surface |
| [ALTIUS_API_REFERENCE.md](./ALTIUS_API_REFERENCE.md) | Human reference (implemented §0 + broader product surface) |
| [ALTIUS_BACKEND_ARCHITECTURE.md](./ALTIUS_BACKEND_ARCHITECTURE.md) | Topology, Keycloak, Postgres, deploy |
| [ALTIUS_DATABASE_DESIGN.md](./ALTIUS_DATABASE_DESIGN.md) | Schema invariants |
| `packages/api-contracts` | TypeScript Zod domain contracts (camelCase) |
| [../deploy/keycloak/README.md](../deploy/keycloak/README.md) | Local realm + M2M (`integration`) setup |

## Authentication

### Interactive (web / mobile)

1. Obtain tokens from **Keycloak** (authorization code + PKCE).
   - Web client: `altius-web`
   - Mobile client: `altius-mobile` (redirect `com.altius.altius_field:/oauthredirect`)
2. Call the API with:

```http
Authorization: Bearer <access_token>
Content-Type: application/json
```

3. Optional hydrate: `GET /api/v3/auth/me`

Password grant (`POST /api/v3/auth/login`) is **off** unless `ALLOW_PASSWORD_GRANT=true` on the API host.

### Machine-to-machine (`integration`)

1. Create a confidential Keycloak client with **Service accounts** enabled (see Keycloak README).
2. Assign realm role **`integration`** to the service account user.
3. Mint a token with `grant_type=client_credentials`, then send the same Bearer header.
4. OpenAPI documents this as `oauth2ClientCredentials`.
5. An org **admin** binds the service-account subject in Postgres:

```
POST /api/v3/integrations
{ "subject": "<kc-service-account-sub>", "display_name": "…", "hub_id": "…" }
GET  /api/v3/integrations
DELETE /api/v3/integrations/{sub}
```

The API maps realm role `integration` → `Role::Integration`. That role may **read** org-scoped allowlist GETs: `/tasks`, `/task/{id}`, `/drivers`, `/hubs`, `/reports`, `/costs`, `/users`. It does **not** unlock writes, `/events`, monitoring, notify, or agent. Tenant comes from membership rows — never an `organization_id` claim. Do not assign `integration` via human user provisioning — use `/integrations`.

See [deploy/keycloak/README.md](../deploy/keycloak/README.md) for the full checklist (template client `altius-integration`).

## Conventions

| Topic | Rule |
|---|---|
| Versioning | Prefix `/api/v3` |
| Resources | Nouns (`/tasks`, `/hubs`); some create verbs retained (`/task-create`) for product compatibility |
| Success body | Often `{ "data": …, "meta": … }` |
| Errors | `{ "error": { "code": <httpStatus>, "message": "…" } }` |
| Wire JSON | **snake_case** is canonical (Rust serialize + OpenAPI). `DeviceEvent` / `DeviceTime` / `Coordinate` also **deserialize** camelCase aliases for `packages/api-contracts` parity (e.g. `eventId` → `event_id`, `occurredAtUtc` → `utc`, `actorId` → `driver_id`). Responses stay snake_case. |
| Tenant | Resolved from JWT membership — do not trust client `tenant_id` for authz |
| Sync | Match event receipts by `event_id` / `server_event_id`, never by array index |
| Skip | `action: skip` requires a non-empty `reason` (route + DB CHECK) |
| Lists | Drivers see assigned tasks only; staff / `integration` see the org board. Cursor pagination is planned |

## Typical flows

### Staff creates a task (web → API)

```
POST /api/v3/task-create
{ Task JSON, snake_case }
→ { "data": { "taskId": "…" } }
```

### Staff deletes a task

```
DELETE /api/v3/task/{id}
→ { "data": { "deleted": "…" } }   // 409 if in_progress
```

### Driver syncs outbox (mobile → API)

```
POST /api/v3/events
[ DeviceEvent, … ]   // 1–500
→ { "data": [ { "status": "accepted", "event_id": "…", "server_event_id": "…" }, … ] }
```

Then `GET /api/v3/tasks` to refresh local board state.

Optional contract fields on events (persisted when present): `schema_version`, `device_sequence`, `expected_task_revision`, `reason`, `observation_id`.

### Daily reports (LHS)

```
POST  /api/v3/reports                 // driver submit (status/revision server-owned)
GET   /api/v3/reports                 // driver: own; staff/integration: org
PATCH /api/v3/reports/{driver}/{day}  // staff review: decision=approved|revision_requested
```

## Retention

| Data | Env | Status |
|---|---|---|
| GPS observations | `MCEASY_RETENTION_HOURS` (McEasy worker) | Implemented |
| Device events | `EVENTS_RETENTION_DAYS` (default `90`; `0` disables; daily prune) | Implemented (V4 index `idx_device_events_occurred`) |

## Viewing the OpenAPI file

- VS Code / Cursor: OpenAPI extension, or paste into [Swagger Editor](https://editor.swagger.io/)
- Developer portal: `pnpm sync:portal-openapi` copies the YAML into `apps/developer-portal`
- CLI: `npx @redocly/cli lint Docs/openapi/altius-ftm-v3.openapi.yaml` (optional)

## Deploy note

Backend runs on a **shared VPS** with other Altius products. Do not restart or reconfigure unrelated Compose stacks. Point Vercel `NEXT_PUBLIC_API_BASE` at the FTM API host only after a dedicated Caddy site / port is provisioned for FTM.
