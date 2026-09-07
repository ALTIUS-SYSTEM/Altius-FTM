# Altius Backend

Rust workspace implementing the production backend for Altius FTM.
See [`Docs/ALTIUS_BACKEND_ARCHITECTURE.md`](../Docs/ALTIUS_BACKEND_ARCHITECTURE.md).

## Crates

| Crate | Purpose |
|---|---|
| `altius-core` | Domain types mirroring `packages/api-contracts`; stop-state machine shared with the mobile `WorkStore` rules |
| `altius-schema` | TypeQL 3.0 schema (`typeql/schema.tql`) + `typedb-driver` connect/migrate |
| `altius-api` | Axum HTTP service: Keycloak JWKS auth, TypeDB persistence, Google Maps client, OpenRouter agent loop |

## Run

```bash
cp .env.example .env   # fill in KEYCLOAK_ISSUER at minimum
cargo run -p altius-api
```

Without `TYPEDB_*` the service starts in demo mode (no persistence).
Without `GOOGLE_MAPS_API_KEY`, `/api/v3/route/eta` returns a schematic
straight-line estimate marked `"maps": "demo"`.
Without `OPENROUTER_API_KEY`, the agent endpoint returns 503.

## Verify

```bash
cargo check --workspace
cargo test --workspace
```

## Endpoints

| Route | Auth | Notes |
|---|---|---|
| `GET /api/v3/health` | — | capability probe |
| `GET /api/v3/tasks` | Bearer | tenant-scoped via Keycloak subject → TypeDB `membership`; stops embedded |
| `GET /api/v3/task/{id}` | Bearer | single task + stops, 404 outside the tenant |
| `POST /api/v3/task-create` | Bearer (non-driver) | upsert task + stops under the caller's hub |
| `POST /api/v3/events` | Bearer (driver) | idempotent ingest: `request-key` dedupe, stop/task stage transition enforced, `recorded`/`reports` links — all in one write tx |
| `POST /api/v3/route/eta` | Bearer | Google Directions, schematic fallback |
| `POST /api/v3/route/optimize` | Bearer | Directions `optimize:true` stop ordering (≤25 waypoints); greedy nearest-neighbor fallback |
| `POST /api/v3/route/geocode` | Bearer | address → coordinate |
| `POST /api/v3/agent/dispatch-suggestion` | Bearer (admin/supervisor) | OpenRouter tool loop; pauses with `awaiting_approval` on gated tools |
| `POST /api/v3/agent/resume` | Bearer (admin/supervisor) | approve/reject a paused call; the loop continues or appends a denial |

## Hardening

- `CORS_ORIGINS` — comma-separated allowlist; empty denies all cross-origin calls.
- Security headers on every response: `nosniff`, `frame DENY`, `referrer-policy`, `default-src 'none'`.
- 2 MiB request-body limit; graceful shutdown on SIGINT/SIGTERM.
- Per-IP rate limiting belongs at the edge/LB.
- Maps calls are client-paced at ~50 QPS per key and retried on 429/5xx (pattern from `google-maps-services-java`).
