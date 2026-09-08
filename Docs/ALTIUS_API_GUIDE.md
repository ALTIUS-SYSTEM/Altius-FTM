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

## Authentication

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

## Conventions

| Topic | Rule |
|---|---|
| Versioning | Prefix `/api/v3` |
| Resources | Nouns (`/tasks`, `/hubs`); some create verbs retained (`/task-create`) for product compatibility |
| Success body | Often `{ "data": …, "meta": … }` |
| Errors | `{ "error": { "code": <httpStatus>, "message": "…" } }` |
| Wire JSON | **snake_case** (Rust). Map from camelCase Zod in adapters |
| Tenant | Resolved from JWT membership — do not trust client `tenant_id` for authz |
| Sync | Match event receipts by `event_id` / `server_event_id`, never by array index |
| Lists | Current task list returns full org set; cursor pagination is planned |

## Typical flows

### Staff creates a task (web → API)

```
POST /api/v3/task-create
{ Task JSON, snake_case }
→ { "data": { "taskId": "…" } }
```

### Driver syncs outbox (mobile → API)

```
POST /api/v3/events
[ DeviceEvent, … ]   // 1–500
→ { "data": [ { "status": "accepted", "event_id": "…", "server_event_id": "…" }, … ] }
```

Then `GET /api/v3/tasks` to refresh local board state.

## Viewing the OpenAPI file

- VS Code / Cursor: OpenAPI extension, or paste into [Swagger Editor](https://editor.swagger.io/)
- CLI: `npx @redocly/cli lint Docs/openapi/altius-ftm-v3.openapi.yaml` (optional)

## Deploy note

Backend runs on a **shared VPS** with other Altius products. Do not restart or reconfigure unrelated Compose stacks. Point Vercel `NEXT_PUBLIC_API_BASE` at the FTM API host only after a dedicated Caddy site / port is provisioned for FTM.
