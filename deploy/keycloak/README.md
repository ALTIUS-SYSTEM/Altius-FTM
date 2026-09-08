# Keycloak realms for Altius FTM

Imported by `docker compose` (`keycloak` service, `--import-realm`).

## Realm files

| File | Used by | Users seeded |
|---|---|---|
| `deploy/keycloak/altius-realm.json` | `docker-compose.yml` (local) | `admin` / `driver`, password `changeme` |
| `deploy/keycloak-prod/altius-realm.json` | `docker-compose.prod.yml` (VPS) | **none** — create the first admin in the console |

The production file also sets `sslRequired: all`, drops the `altius-integration`
dev client and its hard-coded secret, and takes the dashboard redirect URIs from
`${ALTIUS_WEB_ORIGIN}`, substituted by Keycloak at import time.

Both pin each user's `id`, because Keycloak mints the JWT `sub` from it and
`DEFAULT_ADMIN_SUB` has to name that `sub` before the realm has ever been
imported. A username there matches no token and leaves the admin with 403 on
every business endpoint.

## Clients

| Client ID | Type | Redirect / notes |
|-----------|------|------------------|
| `altius-web` | public, PKCE | `http://localhost:3000/callback` (and `127.0.0.1`) |
| `altius-mobile` | public, PKCE | `com.altius.altiusfield:/oauthredirect` |
| `altius-api` | bearer-only | Audience target for API JWT `aud` |
| `altius-integration` | confidential, service accounts | Local M2M template (`client_credentials`); secret in realm JSON for **dev only** |

The marketing landing app does **not** need a Keycloak client or redirect URI. It only deep-links to the web dashboard `/login`; PKCE and `redirect_uri` stay on the web origin.

For deployed web hosts, add matching Valid Redirect URIs / Web Origins / post-logout URIs on `altius-web` (for example `https://<web-host>/callback`).

**Realm import is first-boot only** — `--import-realm` skips a realm that already exists. After the first start, make changes in the admin console or re-import manually; editing this JSON has no effect on a running realm.

## Realm roles (API mapping)

| Realm role | API `Role` | Typical use |
|------------|------------|-------------|
| `super-admin` | `SuperAdmin` | Platform ops |
| `admin` | `Admin` | Org admin / provisioning |
| `supervisor` | `Supervisor` | Hub staff |
| `lead` | `Lead` | Team lead |
| `driver` | `Driver` | Field mobile |
| `integration` | `Integration` | M2M service accounts |

Tokens with **no** recognised realm role are rejected (`403`).

## Machine-to-machine (client credentials)

The API accepts Bearer tokens minted with Keycloak `grant_type=client_credentials` when the access token carries realm role **`integration`**. That role may call org-scoped **GET** allowlist routes only (`/tasks`, `/task/{id}`, `/drivers`, `/hubs`, `/reports`, `/costs`, `/users`). Writes, events, monitoring, and notify stay forbidden.

Local realm import includes template client **`altius-integration`** (service accounts + audience mapper + SA user with `integration`). Production partners get **one confidential client per partner** (do not share the template secret).

### Checklist (per partner / environment)

1. Create a confidential client (or use `altius-integration` locally). Enable **Service accounts**; disable standard/direct flows.
2. Assign realm role **`integration`** to the service-account user (`service-account-<clientId>`).
3. Ensure an audience mapper emits `aud: altius-api`.
4. Look up the service-account **`sub`** (admin console or decode a token).
5. As an org admin, bind membership (Postgres only — no Keycloak Admin from this route):

```bash
curl -s -X POST 'http://127.0.0.1:8080/api/v3/integrations' \
  -H "Authorization: Bearer <admin-token>" \
  -H 'Content-Type: application/json' \
  -d '{"subject":"<service-account-sub>","display_name":"Partner ETL","hub_id":"jakarta"}'
```

Also: `GET /api/v3/integrations`, `DELETE /api/v3/integrations/{sub}` (membership unlink; Keycloak role/client cleanup stays manual).

6. Org scope comes from **membership** (`users` + `user_orgs`); do **not** rely on an `organization_id` claim. One service account → one org.

Token request (local template):

```bash
curl -s -X POST \
  'http://127.0.0.1:8081/realms/altius/protocol/openid-connect/token' \
  -d 'grant_type=client_credentials' \
  -d 'client_id=altius-integration' \
  -d 'client_secret=altius-integration-local-secret'
```

Rotate that secret before any shared environment. OpenAPI documents M2M as `oauth2ClientCredentials`.

**Admin provisioning** (`KEYCLOAK_ADMIN_CLIENT_ID` / `SECRET`) is a separate confidential client used by the API to call Keycloak Admin APIs — not the same as the integrator role above.

## Production / Vercel checklist

1. Keycloak is proxied by Caddy at `{$ALTIUS_DOMAIN}/realms/*` (see root `Caddyfile`). The issuer becomes `https://<domain>/realms/altius` — set `KEYCLOAK_ISSUER` on `api` and `NEXT_PUBLIC_KEYCLOAK_URL` on the web build to `https://<domain>`.
2. `KC_PROXY_HEADERS=xforwarded` is already set in compose so discovery emits `https://` URLs behind TLS-terminating Caddy.
3. In the admin console → `altius-web` client, add:
   - Valid Redirect URIs: `https://<vercel-app>/callback` (plus each preview domain, if previews must log in)
   - Web Origins: `https://<vercel-app>`
   - Valid post-logout redirect URIs: `https://<vercel-app>`
4. Admin console is **not** proxied publicly — reach it via `ssh -L 8081:localhost:8081 <vps>` → `http://localhost:8081/admin`.
5. Persistence: Postgres in every environment. Keycloak gets its own `keycloak` database, created on first init by `deploy/postgres/10-keycloak-db.sh`, so its Liquibase migrations never meet the API's refinery migrations. (The former `KC_DB=dev-file` default did not work — the H2 driver rejected the postgres JDBC URL compose passed alongside it — and stored the realm outside any volume.)
6. For M2M in production: one confidential client per partner + `integration` role; rotate secrets; prefer short-lived client credentials tokens.

Public clients and `altius-integration` include an audience mapper so access tokens carry `aud: altius-api`, matching `KEYCLOAK_AUDIENCE` in compose / backend.

## Local users (dev only)

| Username | Password | Realm role |
|----------|----------|------------|
| `admin` | `changeme` | `admin` |
| `driver` | `changeme` | `driver` |

Change these before any shared or remote environment. Do not reuse in production.
