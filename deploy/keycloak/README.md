# Local Keycloak realm for Altius FTM

Imported by `docker compose` (`keycloak` service, `--import-realm`).

## Clients

| Client ID | Type | Redirect / notes |
|-----------|------|------------------|
| `altius-web` | public, PKCE | `http://localhost:3000/callback` (and `127.0.0.1`) |
| `altius-mobile` | public, PKCE | `com.altius.altius_field:/oauthredirect` |
| `altius-api` | bearer-only | Audience target for API JWT `aud` |

The marketing landing app does **not** need a Keycloak client or redirect URI. It only deep-links to the web dashboard `/login`; PKCE and `redirect_uri` stay on the web origin.

For deployed web hosts, add matching Valid Redirect URIs / Web Origins / post-logout URIs on `altius-web` (for example `https://<web-host>/callback`).

**Realm import is first-boot only** — `--import-realm` skips a realm that already exists. After the first start, make changes in the admin console or re-import manually; editing this JSON has no effect on a running realm.

## Production / Vercel checklist

1. Keycloak is proxied by Caddy at `{$ALTIUS_DOMAIN}/realms/*` (see root `Caddyfile`). The issuer becomes `https://<domain>/realms/altius` — set `KEYCLOAK_ISSUER` on `api` and `NEXT_PUBLIC_KEYCLOAK_URL` on the web build to `https://<domain>`.
2. `KC_PROXY_HEADERS=xforwarded` is already set in compose so discovery emits `https://` URLs behind TLS-terminating Caddy.
3. In the admin console → `altius-web` client, add:
   - Valid Redirect URIs: `https://<vercel-app>/callback` (plus each preview domain, if previews must log in)
   - Web Origins: `https://<vercel-app>`
   - Valid post-logout redirect URIs: `https://<vercel-app>`
4. Admin console is **not** proxied publicly — reach it via `ssh -L 8081:localhost:8081 <vps>` → `http://localhost:8081/admin`.
5. Persistence: local default is `KC_DB=dev-file` (H2). For production set `KC_DB=postgres` (compose passes `KC_DB_URL`/`KC_DB_USERNAME`/`KC_DB_PASSWORD` pointing at the `postgres` service).

Both public clients include an audience mapper so access tokens carry `aud: altius-api`, matching `KEYCLOAK_AUDIENCE` in compose / backend.

## Local users (dev only)

| Username | Password | Realm role |
|----------|----------|------------|
| `admin` | `changeme` | `admin` |
| `driver` | `changeme` | `driver` |

Change these before any shared or remote environment. Do not reuse in production.
