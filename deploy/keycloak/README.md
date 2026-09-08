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

Both public clients include an audience mapper so access tokens carry `aud: altius-api`, matching `KEYCLOAK_AUDIENCE` in compose / backend.

## Local users (dev only)

| Username | Password | Realm role |
|----------|----------|------------|
| `admin` | `changeme` | `admin` |
| `driver` | `changeme` | `driver` |

Change these before any shared or remote environment. Do not reuse in production.
