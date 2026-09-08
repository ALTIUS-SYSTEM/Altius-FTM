# Altius-FTM

**Altius Field Task Manager (FTM)** — Fleet & Transport Management

Product monorepo for Altius FTM: driver mobile app, operations dashboard, marketing landing, shared TypeScript packages, and Rust API.

**Copyright** © 2026 PT Antero Daemon Technologies. Licensed under the Apache License, Version 2.0 — see [LICENSE](./LICENSE).

## Documentation

| Document | Description |
|---|---|
| [ALTIUS_BACKEND_ARCHITECTURE.md](./Docs/ALTIUS_BACKEND_ARCHITECTURE.md) | API topology: Keycloak, Postgres, Maps, agent, deploy |
| [ALTIUS_DATABASE_DESIGN.md](./Docs/ALTIUS_DATABASE_DESIGN.md) | PostgreSQL schema & invariants |
| [ALTIUS_ARCHITECTURE.md](./Docs/ALTIUS_ARCHITECTURE.md) | End-to-end system architecture |
| [ALTIUS_API_REFERENCE.md](./Docs/ALTIUS_API_REFERENCE.md) | REST `/api/v3` reference (implemented + planned) |
| [ALTIUS_API_GUIDE.md](./Docs/ALTIUS_API_GUIDE.md) | Integrator guide (auth, conventions, flows) |
| [openapi/altius-ftm-v3.openapi.yaml](./Docs/openapi/altius-ftm-v3.openapi.yaml) | OpenAPI 3.0.3 (implemented surface) |
| [ALTIUS_UI_BREAKDOWN.md](./Docs/ALTIUS_UI_BREAKDOWN.md) | Mobile & web UI structure |
| [ALTIUS_TENANT_MODEL.md](./Docs/ALTIUS_TENANT_MODEL.md) | Org / hub / role / environments |
| [ALTIUS_PSEUDOCODE.md](./Docs/ALTIUS_PSEUDOCODE.md) | Core operational flows |
| [SECURITY.md](./SECURITY.md) | Vulnerability reporting & security model |
| [Docs/user/](./Docs/user/) | Build-layer engineering prompts |

## Product surface

| Path | Description |
|---|---|
| `apps/mobile` | Flutter driver app — offline-first SQLite, event outbox, Keycloak PKCE, sync |
| `apps/web` | Next.js 15 operations workspace (admin / supervisor / lead) |
| `apps/landing` | Next.js 15 marketing site |
| `packages/api-contracts` | Zod REST/JSON DTOs |
| `packages/algos` | Routing, geofence, GPS comparison, ETA |
| `packages/i18n` | Locales: en, id, th, ja, zh, fil-PH, vi |
| `packages/design-tokens` | Shared design tokens |
| `backend/` | Rust Axum API — Keycloak, Postgres (default), optional TypeDB, Maps, agent |

## Local development

```bash
docker compose up -d          # Keycloak, Postgres, API, Caddy (optional)

pnpm install
cp apps/web/.env.example apps/web/.env.local
pnpm dev:web                  # http://localhost:3000

cd apps/mobile && flutter pub get
flutter run -t lib/main_dev.dart

pnpm typecheck && pnpm test && pnpm lint
cd apps/mobile && flutter analyze && flutter test
cd backend && cargo test --workspace
```

Ports and verification matrix: [AGENTS.md](./AGENTS.md).

## Deploy shape

| Tier | Where | Notes |
|---|---|---|
| Backend | VPS (Docker Compose + Caddy) | API, Postgres, Keycloak; `ALTIUS_DOMAIN`, `CORS_ORIGINS`, `sslmode=require` when needed |
| Web / landing | Vercel | Build-time `NEXT_PUBLIC_API_BASE` + Keycloak URL / realm / `altius-web` |
| Mobile | Store builds | `--dart-define` API + Keycloak issuer; redirect `com.altius.altius_field:/oauthredirect` |

## License

Apache License 2.0 — [LICENSE](./LICENSE).
