# Altius FTM API stress (k6)

End-to-end load harness for `GET/POST /api/v3/*`. This is **not** a unit test:
it hits a live HTTP API (local compose by default).

## Prerequisites

- [k6](https://grafana.com/docs/k6/latest/set-up/install-k6/) (`brew install k6`)
- Local API ready: `GET http://127.0.0.1:8080/api/v3/ready` → 200
- For authenticated scenarios: a Bearer JWT in `ALTIUS_STRESS_TOKEN` (or auto-mint; see Auth)

Local stack:

```bash
docker compose up -d postgres keycloak api
# or from backend/: cargo run -p altius-api
```

## Quick start

```bash
# Health/ready only (no token)
ALTIUS_STRESS_SCENARIO=health ./backend/scripts/stress/run.sh

# Full read mix — run.sh mints a local M2M JWT via Keycloak when possible
./backend/scripts/stress/run.sh

# Or paste a token explicitly
export ALTIUS_STRESS_TOKEN='eyJ…'
./backend/scripts/stress/run.sh

# Tunables
ALTIUS_STRESS_VUS=20 \
ALTIUS_STRESS_DURATION=1m \
ALTIUS_STRESS_HEALTH_RPS=100 \
ALTIUS_STRESS_HEALTH_DUR=20s \
./backend/scripts/stress/run.sh
```

Direct k6 (same env vars):

```bash
k6 run backend/scripts/stress/api-v3.js
```

## Auth

Prefer a **pre-minted** local token. Do not hammer production Keycloak.

| Method | When |
|--------|------|
| `ALTIUS_STRESS_TOKEN` | Always preferred |
| Keycloak `client_credentials` (`altius-integration`) | Default in `run.sh` when no token (`ALTIUS_STRESS_M2M=0` to skip). Local secret only — see `deploy/keycloak/README.md` |
| `ALTIUS_STRESS_USER` + `ALTIUS_STRESS_PASSWORD` → `POST /api/v3/auth/login` | Only if `ALLOW_PASSWORD_GRANT=true` on the API **and** the Keycloak client has Direct Access Grants enabled (off in the shipped realm) |

`run.sh` falls back to **health-only** when no token is available (avoids a guaranteed threshold fail on reads).

Local realm users (dev only): `admin` / `changeme`, `driver` / `changeme`.

M2M mint (local template client, no password grant):

```bash
curl -sS -X POST \
  'http://127.0.0.1:8081/realms/altius/protocol/openid-connect/token' \
  -d 'grant_type=client_credentials' \
  -d 'client_id=altius-integration' \
  -d 'client_secret=altius-integration-local-secret'
# export ALTIUS_STRESS_TOKEN=<access_token>
```

**Integration role note:** M2M tokens cover org-scoped **GET** allowlist routes (`/tasks`, `/hubs`, `/drivers`, …). Bind the service-account `sub` with `POST /api/v3/integrations` (admin) if list endpoints return empty/403. `/auth/me` still exercises JWT + JWKS validation.

Password-grant override (stress-only, **do not** commit compose defaults as `true`):

```bash
# Temporary shell override for the API process / one-shot compose:
ALLOW_PASSWORD_GRANT=true docker compose up -d api
# Enable Direct Access Grants on altius-web in Keycloak admin, then:
ALTIUS_STRESS_M2M=0 ALTIUS_STRESS_USER=admin ALTIUS_STRESS_PASSWORD=changeme \
  ./backend/scripts/stress/run.sh
```

Or paste a token from the web dashboard network tab after signing in at `http://127.0.0.1:3000`.

## Scenarios

| `ALTIUS_STRESS_SCENARIO` | What it does |
|--------------------------|--------------|
| `all` (default) | Health burst, then authenticated GETs |
| `health` | Unauthenticated `/health` + `/ready` arrival-rate burst |
| `reads` | Concurrent `/auth/me`, `/tasks`, `/hubs`, `/drivers`, `/task/{id}` |
| `writes` | Only if `ALTIUS_STRESS_WRITES=1`: `POST /task-create` with `stress-*` ids (local only) |

Writes stay **opt-in**. The runner refuses non-localhost bases unless `ALTIUS_STRESS_ALLOW_REMOTE=1`.

## Metrics

k6 prints RPS, latency percentiles, and failure rate. JSON summary lands in
`backend/scripts/stress/out/summary-*.json` (gitignored via the `out/` directory
convention — add results there, do not commit secrets).

Custom trends (authenticated reads):

| Metric | Meaning |
|--------|---------|
| `altius_auth_cold_ms` | First ~`ALTIUS_STRESS_COLD_ITERS` VU iterations (default 20, split across VUs) |
| `altius_auth_warm_ms` | Steady-state after the cold window |
| `altius_latency_ms` | All recorded requests |
| Request tags | `auth_phase=cold\|warm` on authenticated GETs |

Compare cold vs warm p95: a large gap usually means JWKS fetch / TLS to Keycloak / first Postgres plan, not steady load.

## Bottlenecks

Once auth is wired, watch these before chasing application code:

1. **JWKS / token validation (cold path)** — The API fetches Keycloak JWKS on first validated Bearer request (and again after cache expiry). Expect p95 spikes on the **first** authenticated requests after API or Keycloak restart. Use `altius_auth_cold_ms` vs `altius_auth_warm_ms` (or tag `auth_phase`) to separate cold from steady-state; do not treat a cold p95 spike alone as a Postgres regression.
2. **Postgres list endpoints** — Default read mix hits `/tasks`, `/hubs`, `/drivers` (and `/task/{id}` when ids exist). Rising warm p95 or timeouts here usually point at query plans, connection pool saturation, or missing indexes — not JWKS.
3. **Maps routes excluded** — `/route/*` and static-map proxies are intentionally **out** of the default mix. They call external Google Maps (QPS, cost, and latency noise). Stress them in a dedicated scenario with a key and budget, not in nightly defaults.
4. **Interpreting first-auth spikes** — After `docker compose up` or `cargo run`, the first wave of authenticated traffic pays JWKS + possibly DNS/TLS to Keycloak. If cold p95 is high but warm p95 is flat under the threshold, auth cold-start is expected; if **warm** p95 climbs with VU count, investigate Postgres and handler time next.
5. **Auth failures** — Rising `401` / `altius_auth_failures` → bad/expired token, `aud`/`iss` mismatch, or Keycloak unreachable for JWKS — fix minting before load tuning.

## CI / nightly suggestions

- **PR CI:** keep using `cargo test` / `pg_it` only; do not add k6 against a shared
  environment on every PR.
- **Nightly (optional):** compose `postgres` + `keycloak` + `api`, mint a token once
  (M2M or pre-minted), run `ALTIUS_STRESS_SCENARIO=all` with modest VUs (10 × 2–5 min),
  upload the summary JSON as an artifact, fail on `http_req_failed > 5%` or warm p95 >
  budget (ignore isolated cold spikes if warm is healthy).
- Gate the job on `ubuntu-latest` + `setup-k6`; never target production URLs.
- Keep `ALLOW_PASSWORD_GRANT=false` in committed compose defaults; use a temporary
  env override only on disposable local/CI runners when password grant is required.
