# Altius FTM — Agent Notes

## Verification matrix

Run these before claiming a change is ready:

```bash
# Mobile
flutter analyze
flutter test

# TypeScript / Node
pnpm -r typecheck
pnpm -r test
pnpm -r lint

# Rust
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Local stack

```bash
docker compose up -d
```

- Keycloak: http://localhost:8081
- PostgreSQL: localhost:5432 (`postgres://altius:altius@localhost:5432/altius`)
- TypeDB: localhost:1729 (optional, only when `STORE_BACKEND=typedb`)
- API: http://localhost:8080
- Web dashboard: http://localhost:3000
- Landing (compose): http://localhost:3001
- Landing (`pnpm --filter @altius/landing dev`): http://127.0.0.1:3100

## Auth E2E (local)

Keycloak imports `deploy/keycloak/altius-realm.json` on compose start (`altius-web`, `altius-mobile`, audience `altius-api`).

**Landing → web handoff:** marketing CTAs link to `{NEXT_PUBLIC_WEB_APP_URL}/login` on `apps/web`. OAuth/PKCE starts only on the web origin (Keycloak redirect URI remains `…/callback` on the web app, not the landing site). Copy `apps/landing/.env.example` → `.env.local` and set `NEXT_PUBLIC_WEB_APP_URL` (default `http://127.0.0.1:3000`).

```bash
# Web — copy apps/web/.env.example → .env.local, then:
pnpm --filter @altius/web dev
# Sign in at http://127.0.0.1:3000 → Keycloak (admin/changeme or driver/changeme)

# Landing → dashboard (optional):
# copy apps/landing/.env.example → .env.local, then:
pnpm --filter @altius/landing dev
# Open http://127.0.0.1:3100 → Log in / Start Dispatching → web /login

# Mobile live against local stack:
cd apps/mobile && flutter run -t lib/main_dev.dart \
  --dart-define=API_BASE=http://127.0.0.1:8080 \
  --dart-define=KEYCLOAK_ISSUER=http://127.0.0.1:8081/realms/altius \
  --dart-define=KEYCLOAK_CLIENT_ID=altius-mobile \
  --dart-define=KEYCLOAK_REDIRECT_URI=com.altius.altius_field:/oauthredirect
```

Without dart-defines, `main_dev` keeps the local demo workspace path (no IdP).
`main_prod` requires Keycloak (`demoWorkspace: false`).

## Key decisions

- Mobile auth uses `flutter_appauth` PKCE against Keycloak.
- Tokens live in `FlutterSecureStorage`, never in the SQLite `preferences` table.
- Operational PII (task addresses, GPS events, reports) remains in app-private
  unencrypted SQLite until SQLCipher; Android `allowBackup` is off. See F-06-05.
- Sync matches event receipts by `event_id` / `server_event_id`, never by array index.
- Maps key is server-side only; mobile and web get a backend-proxied static map.
- LHS is a mobile-only form with clipboard and native share; no third-party SMS/WhatsApp vendor is wired yet.
- Prod Postgres: `DATABASE_URL` with `sslmode=require`; local compose uses cleartext.
