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
- Landing: http://localhost:3001

## Key decisions

- Mobile auth uses `flutter_appauth` PKCE against Keycloak.
- Tokens live in `FlutterSecureStorage`, never in the SQLite `preferences` table.
- Sync matches event receipts by `event_id` / `server_event_id`, never by array index.
- Maps key is server-side only; mobile and web get a backend-proxied static map.
- LHS is a mobile-only form with clipboard and native share; no third-party SMS/WhatsApp vendor is wired yet.
