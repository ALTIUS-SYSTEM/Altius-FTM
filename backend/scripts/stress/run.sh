#!/usr/bin/env bash
# Run Altius FTM /api/v3 k6 stress against a local (or env-targeted) API.
# Defaults to http://127.0.0.1:8080 — refuse obvious production hosts unless
# ALTIUS_STRESS_ALLOW_REMOTE=1 is set.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
BASE="${ALTIUS_STRESS_BASE_URL:-http://127.0.0.1:8080}"
BASE="${BASE%/}"
API="${BASE}/api/v3"
SUMMARY_DIR="${ALTIUS_STRESS_OUT_DIR:-$ROOT/out}"
mkdir -p "$SUMMARY_DIR"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
SUMMARY_JSON="${ALTIUS_STRESS_SUMMARY_JSON:-$SUMMARY_DIR/summary-$STAMP.json}"

if ! command -v k6 >/dev/null 2>&1; then
  echo "k6 not found. Install: https://grafana.com/docs/k6/latest/set-up/install-k6/" >&2
  exit 127
fi

case "$BASE" in
  http://127.0.0.1:*|http://localhost:*|http://\[::1\]:*)
    ;;
  *)
    if [[ "${ALTIUS_STRESS_ALLOW_REMOTE:-}" != "1" ]]; then
      echo "Refusing non-local base URL: $BASE" >&2
      echo "Set ALTIUS_STRESS_ALLOW_REMOTE=1 only for intentional remote staging." >&2
      exit 2
    fi
    ;;
esac

echo "==> Probing $API/ready …"
ready_code="$(curl -s -o /dev/null -w '%{http_code}' --connect-timeout 2 "$API/ready" || true)"
if [[ "$ready_code" != "200" ]]; then
  echo "API not up (GET $API/ready → HTTP ${ready_code:-000})." >&2
  echo "Start local stack, then re-run:" >&2
  echo "  docker compose up -d postgres keycloak api" >&2
  echo "  # or: cd backend && cargo run -p altius-api" >&2
  echo "Script is ready at $ROOT/api-v3.js" >&2
  exit 3
fi

# Optional token mint: prefer explicit JWT, else API password grant (local only).
if [[ -z "${ALTIUS_STRESS_TOKEN:-}" && -n "${ALTIUS_STRESS_USER:-}" && -n "${ALTIUS_STRESS_PASSWORD:-}" ]]; then
  echo "==> Minting token via POST $API/auth/login (ALLOW_PASSWORD_GRANT must be true)…"
  tok_json="$(curl -sS -X POST "$API/auth/login" \
    -H 'Content-Type: application/json' \
    -d "{\"username\":\"${ALTIUS_STRESS_USER}\",\"password\":\"${ALTIUS_STRESS_PASSWORD}\"}" || true)"
  ALTIUS_STRESS_TOKEN="$(printf '%s' "$tok_json" | sed -n 's/.*"access_token"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')"
  if [[ -z "$ALTIUS_STRESS_TOKEN" ]]; then
    echo "Login failed or password grant disabled. Body (truncated): ${tok_json:0:240}" >&2
  fi
fi

SCENARIO="${ALTIUS_STRESS_SCENARIO:-all}"
if [[ -z "${ALTIUS_STRESS_TOKEN:-}" && ( "$SCENARIO" == "all" || "$SCENARIO" == "reads" || "$SCENARIO" == "writes" ) ]]; then
  echo "No ALTIUS_STRESS_TOKEN — falling back to health-only (Keycloak/password grant not used)." >&2
  SCENARIO=health
fi

export ALTIUS_STRESS_BASE_URL="$BASE"
export ALTIUS_STRESS_TOKEN="${ALTIUS_STRESS_TOKEN:-}"
export ALTIUS_STRESS_SUMMARY_JSON="$SUMMARY_JSON"
export ALTIUS_STRESS_VUS="${ALTIUS_STRESS_VUS:-10}"
export ALTIUS_STRESS_DURATION="${ALTIUS_STRESS_DURATION:-30s}"
export ALTIUS_STRESS_HEALTH_RPS="${ALTIUS_STRESS_HEALTH_RPS:-50}"
export ALTIUS_STRESS_HEALTH_DUR="${ALTIUS_STRESS_HEALTH_DUR:-15s}"
export ALTIUS_STRESS_SCENARIO="$SCENARIO"
export ALTIUS_STRESS_WRITES="${ALTIUS_STRESS_WRITES:-0}"
export ALTIUS_STRESS_USER="${ALTIUS_STRESS_USER:-}"
export ALTIUS_STRESS_PASSWORD="${ALTIUS_STRESS_PASSWORD:-}"

echo "==> k6 run (scenario=${ALTIUS_STRESS_SCENARIO} vus=${ALTIUS_STRESS_VUS} duration=${ALTIUS_STRESS_DURATION})"
echo "    summary → $SUMMARY_JSON"
k6 run "$ROOT/api-v3.js"
echo "==> Done."
