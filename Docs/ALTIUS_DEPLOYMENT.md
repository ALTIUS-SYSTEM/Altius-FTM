# Altius FTM — VPS deployment runbook

Backend on a single VPS (`api` + `postgres` + `keycloak` + `caddy`); `apps/web`
and `apps/landing` on Vercel. Mobile builds point at the same public API.

Every command below was exercised against the real stack locally. Where a step
has a known failure mode, it is stated inline rather than left to be discovered.

---

## 1. What you need before you start

| Item | Notes |
|---|---|
| VPS | 2 vCPU / 4 GB RAM minimum. Keycloak alone idles around 500 MB, and the Rust build peaks well above 2 GB. |
| Disk | 40 GB. Images total ~1.5 GB; Postgres and Keycloak grow with use. |
| OS | Debian 12 / Ubuntu 22.04+ with Docker Engine 24+ and the Compose v2 plugin. The overlay uses `!override`, which needs Compose **v2.24+**. |
| DNS | `A` record for your domain pointing at the VPS **before first boot** — Caddy's ACME challenge fails without it and backs off. |
| Ports | 80 and 443 open inbound. Nothing else needs to be. |
| Vercel | A project for `apps/web` (and optionally `apps/landing`). |

Architecture note: build on the VPS, not on a laptop. An image built on Apple
silicon is `arm64` and will not run on an `amd64` VPS.

---

## 2. Prepare the server

```bash
sudo apt update && sudo apt install -y docker.io docker-compose-v2 git
sudo usermod -aG docker "$USER"   # log out and back in for this to take effect
docker compose version            # must be v2.24 or newer
```

Firewall — only the edge is public. The API (8080) and Keycloak (8081) bind to
loopback in the production overlay and are reached over an SSH tunnel:

```bash
sudo ufw allow OpenSSH && sudo ufw allow 80/tcp && sudo ufw allow 443/tcp && sudo ufw enable
```

---

## 3. Clone and configure

```bash
git clone https://github.com/ALTIUS-SYSTEM/Altius-FTM.git
cd Altius-FTM
git checkout production

# Generates .env with fresh secrets, made on this host so they never travel.
# The dashboard origin is optional — see §6b to add it later.
./deploy/bootstrap-env.sh ftm.example.com https://app.vercel.app
```

What that writes, and why each value matters:

| Variable | Value | Why it matters |
|---|---|---|
| `ALTIUS_DOMAIN` | `ftm.example.com` | Hostname only — no scheme, no path. Drives Caddy's TLS and Keycloak's `KC_HOSTNAME`. |
| `ALTIUS_WEB_ORIGIN` | `https://app.vercel.app` | The **dashboard's** origin, not this host. Seeds the realm's redirect URIs on first boot. Optional — omit it and fix it later with `deploy/set-web-origin.sh` (§6b). |
| `KEYCLOAK_ISSUER` | `https://ftm.example.com/realms/altius` | Must match the token's `iss` **exactly**. A token minted via a different hostname is rejected with 401. |
| `CORS_ORIGINS` | `https://app.vercel.app` | Empty denies every cross-origin call, so the Vercel dashboard cannot reach the API at all. |
| `POSTGRES_PASSWORD` | generated | — |
| `DATABASE_URL` | same user/password as above | It does **not** inherit `POSTGRES_PASSWORD`; compose falls back to a literal `postgres://altius:altius@…`. Mismatch ⇒ the API crash-loops. |
| `KEYCLOAK_ADMIN_PASSWORD` | generated | Console bootstrap admin. |
| `AGENT_STATE_SECRET` | generated | Absent ⇒ agent resume refuses. |

Leave `DEFAULT_ADMIN_SUB` at its default for now — §5 sets it to a real user.

---

## 4. First boot

```bash
./deploy/preflight.sh          # refuses conditions that are known to fail
docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d --build --wait
```

`preflight.sh` checks the Compose version (the overlay needs ≥ 2.24 for
`!override`), that the domain resolves to this host, that 80/443 are free, that
RAM+swap can survive the Rust build, and that `.env` is internally consistent.

The Rust release build takes 5–15 minutes on a 2 vCPU box, and **needs more than
4 GB of RAM+swap** — below that the linker is OOM-killed and reports only
`signal: 9`. Add swap first if the box is small:

```bash
fallocate -l 4G /swapfile && chmod 600 /swapfile && mkswap /swapfile && swapon /swapfile
echo '/swapfile none swap sw 0 0' >> /etc/fstab
```

The overlay starts only `api`, `postgres`, `keycloak`, `caddy` — `web`,
`landing`, and `typedb` are held behind inactive profiles.

Check it:

```bash
docker compose -f docker-compose.yml -f docker-compose.prod.yml ps
curl -fsS https://ftm.example.com/healthz                 # -> ok
curl -fsS https://ftm.example.com/api/v3/ready            # -> {"ready":true,...}
curl -fsS https://ftm.example.com/realms/altius/.well-known/openid-configuration \
  | grep -o '"issuer":"[^"]*"'                            # must equal KEYCLOAK_ISSUER
```

If `ready` returns 503, persistence is not up — check `docker compose logs api`.

**The realm is imported on first boot only.** `--import-realm` skips a realm
that already exists, so later edits to `deploy/keycloak-prod/altius-realm.json`
do nothing; change those settings in the admin console instead.

---

## 5. Create the first admin

The production realm deliberately seeds **no users** — unlike the dev realm,
which ships `admin`/`driver` with the password `changeme`.

The admin console is not exposed publicly. Reach it over SSH:

```bash
ssh -L 8081:127.0.0.1:8081 user@vps
# then open http://localhost:8081/admin — sign in with KEYCLOAK_ADMIN / KEYCLOAK_ADMIN_PASSWORD
```

1. Realm `altius` → **Users** → *Add user*, set a password (uncheck Temporary if
   you prefer), and assign the realm role `admin`.
2. Copy that user's **ID** — a UUID. This is the `sub` claim their tokens carry.
3. Put it in `.env` and restart the API:

```bash
# .env
DEFAULT_ADMIN_SUB=<the UUID you copied>
```

```bash
docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d api
```

`link_admin_user` runs on every start and is idempotent, so this links the real
user to the bootstrap org and hub.

> Skipping this step is the most likely way to end up with a deployment where
> login succeeds but **every endpoint returns 403**. The API resolves tenancy by
> matching the JWT `sub` against `user_orgs`; a username there matches nothing.

---

## 6. Deploy web and landing to Vercel

`NEXT_PUBLIC_*` are inlined at **build** time. Setting them afterwards has no
effect on an already-built deployment — you must redeploy.

For the `apps/web` project (root directory `apps/web`, framework Next.js):

| Variable | Value |
|---|---|
| `NEXT_PUBLIC_API_BASE` | `https://ftm.example.com` |
| `NEXT_PUBLIC_KEYCLOAK_URL` | `https://ftm.example.com` |
| `NEXT_PUBLIC_KEYCLOAK_REALM` | `altius` |
| `NEXT_PUBLIC_KEYCLOAK_CLIENT_ID` | `altius-web` |

`apps/web/src/middleware.ts` builds the CSP `connect-src` from
`NEXT_PUBLIC_API_BASE` and `NEXT_PUBLIC_KEYCLOAK_URL`. If either is unset at
build time, the browser blocks the API and token calls and the dashboard fails
with no obvious server-side error.

For `apps/landing`: `NEXT_PUBLIC_WEB_APP_URL` = the dashboard origin, and
optionally `NEXT_PUBLIC_DOCS_URL`.

### 6b. Point the backend at the dashboard origin

Three places have to agree, or login fails in ways that look like a bug: the
Keycloak `altius-web` client, the API's `CORS_ORIGINS`, and its
`KEYCLOAK_REDIRECT_URI`. One command sets all three and restarts the API:

```bash
./deploy/set-web-origin.sh https://app.vercel.app
# several origins, e.g. to allow preview deployments:
./deploy/set-web-origin.sh https://app.vercel.app https://staging.example.com
```

Run it whenever the origin is first known or later changes. It edits the
existing client in place — it does **not** recreate the realm and does not touch
users, so deferring the origin past the first boot costs nothing. Safe to re-run.

---

## 7. Verify end to end

```bash
curl -fsS https://ftm.example.com/api/v3/health
curl -fsS https://ftm.example.com/api/v3/ready
curl -s -o /dev/null -w '%{http_code}\n' https://ftm.example.com/            # 404 — no web upstream here
curl -s -o /dev/null -w '%{http_code}\n' https://ftm.example.com/admin/      # 404 — console is not proxied
curl -s -o /dev/null -w '%{http_code}\n' https://ftm.example.com/api/v3/tasks # 401 — unauthenticated
```

Then sign in to the Vercel dashboard as the admin from §5 and confirm the task
list loads. A 403 there means `DEFAULT_ADMIN_SUB` does not match that user's ID.

---

## 7b. When the host already has a reverse proxy

`docker-compose.prod.yml` brings up its own Caddy on :80/:443. If something
else on the host already owns those ports, add `docker-compose.sharededge.yml`
as well: it drops our Caddy and publishes `api` and `keycloak` on loopback
(8096/8097 by default, overridable with `ALTIUS_API_PORT` / `ALTIUS_KC_PORT`).

```bash
docker compose -f docker-compose.yml -f docker-compose.prod.yml \
               -f docker-compose.sharededge.yml up -d --build --wait
docker network connect altius-ftm_default <their-proxy-container>
```

Then route `/api/v3/*` to `altius-ftm-api-1:8080` and `/realms/*` plus
`/resources/*` to `altius-ftm-keycloak-1:8081`, leaving `/admin` and
`/metrics` unproxied.

Changing a proxy config that other services depend on deserves a procedure,
not a quick edit:

1. Record how every existing site responds, so "unchanged" can be proven.
2. Copy the config aside, and write the rollback script **before** the change.
3. Edit a candidate copy; `caddy validate` it; only then move it into place.
4. `caddy reload`, never restart — a reload is graceful and a config that
   fails validation never reaches the running server.
5. Re-check every existing site against step 1, then the new one.
6. Run the rollback once and re-apply, so the undo path is tested rather than
   assumed.

Note what this topology gives up: `Caddyfile.backend` rate-limits
`/api/v3/auth/*` to 10/min, and a stock Caddy image has no rate-limit module.
Behind someone else's proxy, that protection is theirs to provide.

## 8. Operations

```bash
# logs
docker compose -f docker-compose.yml -f docker-compose.prod.yml logs -f api

# update to the latest production branch
git pull origin production
docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d --build --wait

# database backup (Keycloak's realm lives in its own database — back up both)
docker compose exec -T postgres pg_dump -U altius altius   > altius-$(date +%F).sql
docker compose exec -T postgres pg_dump -U altius keycloak > keycloak-$(date +%F).sql
```

Roll back by checking out the previous commit and rebuilding. Database
migrations (refinery) are forward-only — a rollback across a migration needs a
restore from the dump above.

### Failure modes worth recognising

| Symptom | Cause |
|---|---|
| `api` crash-loops on boot | `DATABASE_URL` credentials disagree with `POSTGRES_PASSWORD`. |
| Login works, every endpoint 403 | `DEFAULT_ADMIN_SUB` is not that user's Keycloak UUID (§5). |
| 401 on a token that looks valid | Token `iss` ≠ `KEYCLOAK_ISSUER`; usually reaching Keycloak by IP instead of the domain. |
| Dashboard shows a config error | `NEXT_PUBLIC_API_BASE` was not set **at Vercel build time**. |
| Browser blocks API calls, no server error | Same cause — the CSP `connect-src` was built without those origins. |
| "invalid redirect" at login | `altius-web` redirect URIs do not include the dashboard origin — run `deploy/set-web-origin.sh`. |
| Caddy will not start | `ALTIUS_DOMAIN` set to an empty string. It must be a hostname or absent. |
| Rust build dies with `signal: 9` | OOM. Add swap (§4) — 4 GB of RAM alone is not enough. |
| Host disk fills up | Pre-dates the `logging` caps in compose; prune with `docker system prune -a --volumes` after backing up. |
| Certificate never issues | DNS not pointing at the VPS, or 80/443 closed. |

---

## 9. Security posture as deployed

- Postgres publishes no host port; Keycloak and the API bind to loopback only.
- The Keycloak admin console and `/metrics` are not proxied — SSH tunnel only.
- Caddy rate-limits `/api/v3/auth/*` at 10/min, `/api/v3/*` at 120/min,
  `/realms/*` at 60/min, keyed on the peer address with forwarded IPs untrusted.
- HSTS, `X-Frame-Options: DENY`, `nosniff`, and a strict Referrer-Policy on the
  public domain; the API and web app set their own CSP.
- `ALLOW_PASSWORD_GRANT=false` — PKCE only. Containers run with
  `no-new-privileges` and dropped capabilities; web/landing run read-only.
- The production realm seeds no users and sets `sslRequired: all`.
