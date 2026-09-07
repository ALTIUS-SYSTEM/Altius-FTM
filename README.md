# Altius-FTM

**Altius Field Task Manager (FTM)** — Fleet & Transport Management

Reverse-engineered technical architecture documentation for the Altius FTM mobile application (`app.paket.mile_field` v1.40.8), a Flutter-based field worker management platform developed by PT. Paket Informasi Digital.

## Documentation

All documentation is in the [`Docs/`](./Docs/) directory:

| Document | Description |
|---|---|
| [ALTIUS_ARCHITECTURE.md](./Docs/ALTIUS_ARCHITECTURE.md) | Comprehensive technical architecture (backend, frontend, DB, E2E flow, high & low level architecture) |
| [ALTIUS_UI_BREAKDOWN.md](./Docs/ALTIUS_UI_BREAKDOWN.md) | Mobile & web UI structure (68 routes, 40+ pages, 13 component types, 30+ cubits, 614 i18n keys) |
| [ALTIUS_API_REFERENCE.md](./Docs/ALTIUS_API_REFERENCE.md) | API endpoint reference (auth, task, flow, data, sync, webhooks, media) |
| [ALTIUS_TENANT_MODEL.md](./Docs/ALTIUS_TENANT_MODEL.md) | Tenant hierarchy (8 build flavors, runtime org switching, hub geofencing, permissions) |
| [ALTIUS_PSEUDOCODE.md](./Docs/ALTIUS_PSEUDOCODE.md) | Pseudocode for 19 core flows (login, check-in, task execution, sync, org switch, QR login) |

## Extraction Artifacts

Raw extraction artifacts are in [`extraction/mile_field/`](./extraction/mile_field/):

- `base.apk` + split APKs (arm64, en, in, ms, xxhdpi)
- `extracted/` — decoded manifest, DEX files, native libraries, Flutter assets, environment files, string inventories
- `ARCHITECTURE.md` / `REPORT.md` — original extraction reports

## Summary

| Attribute | Value |
|---|---|
| Package | `app.paket.mile_field` |
| Version | 1.40.8 (build 3790) |
| Framework | Flutter (Dart AOT, arm64) |
| Architecture | Clean Architecture + Cubit (Bloc) |
| Features | 29 feature modules, 114 use cases, 60+ entities |
| Tenants | 8 build flavors, runtime org switching, hub geofencing |
| Languages | 7 (en, id, th, ja, zh, fil-PH, vi) |
| Backend | REST API at `/api/v3` (apiweb.mile.app) |
| Storage | SQLite (local) + AWS S3 ap-southeast-1 (media) |

## Implementation

The workspace contains a rebranded demo implementation under `apps/` and `packages/`:

| Path | Description |
|---|---|
| `apps/web` | Next.js 15 operations workspace (admin/supervisor/lead). Synthetic localStorage demo — no backend, no authentication, no real maps. |
| `apps/mobile` | Flutter driver app. Offline-first SQLite store (drift) with immutable event outbox, 3-tap arrival/activity/completion flow, daily report (LHS), Dijkstra demo routing, geofence/GPS-anomaly simulations. |
| `packages/api-contracts` | Zod schemas for REST/JSON DTOs (tasks, LHS, location, identity, envelopes). |
| `packages/algos` | Dijkstra, haversine, corridor geofence with hysteresis, GPS stream comparison, ETA, daily aggregation. |
| `packages/i18n` | 7 locale catalogs (en, id, th, ja, zh, fil-PH, vi). |
| `packages/design-tokens` | Design tokens from `Docs/asset/DESIGN.md`. |
| `apps/landing` | Next.js 15 marketing landing (React Bits sections, Altius theme). |
| `backend/` | Rust workspace: Axum API + Keycloak OIDC + TypeDB + Google Maps + OpenRouter agent. See [ALTIUS_BACKEND_ARCHITECTURE.md](./Docs/ALTIUS_BACKEND_ARCHITECTURE.md). |

### Development

```bash
# Web (requires Node 22.14+ and pnpm 10)
pnpm install
pnpm dev:web        # http://localhost:3000 → demo login → dashboard

# Mobile (requires Flutter 3.44+)
cd apps/mobile
flutter pub get
flutter run -t lib/main_dev.dart

# Quality gates
pnpm typecheck      # all workspace packages
pnpm test           # package unit tests + web vitest
pnpm lint           # eslint (ts, react-hooks, jsx-a11y)
pnpm qa:audit       # dependency audit
cd apps/mobile && flutter analyze && flutter test
```

### Demo boundaries

- **No backend.** Web data lives in `localStorage`; mobile data lives in on-device SQLite. Nothing is uploaded.
- **No real maps/GPS.** Map panels are offline schematics. Dijkstra runs on a fixed demo graph — not road routing. GPS anomaly and geofence checks use fixture streams and are labeled "simulation only."
- **No authentication.** Login is a persona selector; OTP/authenticator flows are stubs pending a real identity provider.
- Production integration gates: REST endpoints, Google Maps/Directions key, vehicle telematics feed, background location, server-side anomaly adjudication.

## License

See [LICENSE](./LICENSE).
