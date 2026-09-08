# Altius FTM API stress (k6)

End-to-end load harness for `GET/POST /api/v3/*`. This is **not** a unit test:
it hits a live HTTP API (local compose by default).

## Prerequisites

- [k6](https://grafana.com/docs/k6/latest/set-up/install-k6/) (`brew install k6`)
- Local API ready: `GET http://127.0.0.1:8080/api/v3/ready` → 200
- For authenticated scenarios: a Bearer JWT in `ALTIUS_STRESS_TOKEN`

Local stack:

```bash
docker compose up -d postgres keycloak api
# or from backend/: cargo run -p altius-api
```

## Quick start

```bash
# Health/ready only (no token)
ALTIUS_STRESS_SCENARIO=health ./backend/scripts/stress/run.sh

# Full read mix — mint a local JWT first (see Auth below)
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
| `ALTIUS_STRESS_USER` + `ALTIUS_STRESS_PASSWORD` → `POST /api/v3/auth/login` | Only if `ALLOW_PASSWORD_GRANT=true` on the API **and** the Keycloak client has direct access grants enabled (off in the shipped realm) |

`run.sh` falls back to **health-only** when no token is available (avoids a guaranteed threshold fail on reads).

Local realm users (dev only): `admin` / `changeme`, `driver` / `changeme`.

Example (stress-only, local compose): set `ALLOW_PASSWORD_GRANT=true` for the API container, enable Direct Access Grants on `altius-web` in the Keycloak admin console, then:

```bash
ALTIUS_STRESS_USER=admin ALTIUS_STRESS_PASSWORD=changeme ./backend/scripts/stress/run.sh
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

Watch for bottlenecks:

- Rising `401` / auth failures → JWKS / token mint / `aud` mismatch
- High p95 on `/tasks` / `/hubs` → Postgres
- Maps routes (`/route/*`) are **not** in the default mix (external QPS + cost)

## CI / nightly suggestions

- **PR CI:** keep using `cargo test` / `pg_it` only; do not add k6 against a shared
  environment on every PR.
- **Nightly (optional):** compose `postgres` + `keycloak` + `api`, mint a token once,
  run `ALTIUS_STRESS_SCENARIO=all` with modest VUs (10 × 2–5 min), upload the
  summary JSON as an artifact, fail on `http_req_failed > 5%` or p95 > budget.
- Gate the job on `ubuntu-latest` + `setup-k6`; never target production URLs.
