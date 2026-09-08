# Altius-FTM

**Altius Field Task Manager (FTM)** — Fleet & Transport Management

Product workspace for the Altius FTM platform: driver mobile app, operations dashboard, marketing landing, shared contracts/algorithms, and Rust API.

## Documentation

| Document | Description |
|---|---|
| [ALTIUS_BACKEND_ARCHITECTURE.md](./Docs/ALTIUS_BACKEND_ARCHITECTURE.md) | Production backend topology (API, Keycloak, Postgres, Maps, agent) |
| [ALTIUS_ARCHITECTURE.md](./Docs/ALTIUS_ARCHITECTURE.md) | System architecture (clients, API, data, E2E flows) |
| [ALTIUS_API_REFERENCE.md](./Docs/ALTIUS_API_REFERENCE.md) | REST `/api/v3` endpoint reference |
| [ALTIUS_UI_BREAKDOWN.md](./Docs/ALTIUS_UI_BREAKDOWN.md) | Mobile & web UI structure |
| [ALTIUS_TENANT_MODEL.md](./Docs/ALTIUS_TENANT_MODEL.md) | Org / hub / role model and environments |
| [ALTIUS_PSEUDOCODE.md](./Docs/ALTIUS_PSEUDOCODE.md) | Core operational flows (login, check-in, sync, reports) |
| [Docs/user/](./Docs/user/) | Build-layer prompts (design → deploy) |

## Product surface

| Path | Description |
|---|---|
| `apps/mobile` | Flutter driver app — offline-first SQLite, event outbox, Keycloak PKCE, sync to API |
| `apps/web` | Next.js 15 operations workspace (admin / supervisor / lead) |
| `apps/landing` | Next.js 15 marketing site |
| `packages/api-contracts` | Zod schemas for REST/JSON DTOs |
| `packages/algos` | Routing / geofence / GPS comparison / ETA helpers |
| `packages/i18n` | Locale catalogs (en, id, th, ja, zh, fil-PH, vi) |
| `packages/design-tokens` | Shared design tokens |
| `backend/` | Rust Axum API — Keycloak OIDC, Postgres (default), optional TypeDB, Maps, OpenRouter agent |

### Local development

```bash
# Stack (Keycloak, Postgres, API, optional Caddy)
docker compose up -d

# Web (Node 22.14+, pnpm 10)
pnpm install
cp apps/web/.env.example apps/web/.env.local   # point at API + Keycloak
pnpm dev:web        # http://localhost:3000

# Mobile (Flutter 3.44+)
cd apps/mobile
flutter pub get
flutter run -t lib/main_dev.dart
# Live IdP: pass --dart-define=API_BASE=... KEYCLOAK_*=...

# Quality gates
pnpm typecheck && pnpm test && pnpm lint
cd apps/mobile && flutter analyze && flutter test
cd backend && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
```

See [AGENTS.md](./AGENTS.md) for ports, auth decisions, and the verification matrix.

### Deploy shape

- **Backend** — VPS (Docker Compose + Caddy TLS): API, Postgres, Keycloak
- **Frontend** — Vercel: `apps/web` and `apps/landing` with `NEXT_PUBLIC_API_BASE` / Keycloak env at build time
- **Mobile** — store builds against the public API + Keycloak issuer

## License

See [LICENSE](./LICENSE).
