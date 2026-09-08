#!/usr/bin/env bash
# Pre-deployment checks for the backend-only VPS topology.
#
#   ./deploy/preflight.sh                 # reads ALTIUS_DOMAIN from .env
#   ./deploy/preflight.sh ftm.example.com
#
# Each check here corresponds to a failure that is slow or confusing to
# diagnose after the fact — an OOM-killed Rust build reports only "killed",
# a too-old Compose silently mis-merges the overlay, and a domain that does
# not resolve makes Caddy retry ACME on a back-off for hours.
#
# Exit codes: 0 all clear · 1 blocking problem · 2 warnings only.
set -uo pipefail

cd "$(dirname "$0")/.."

FAIL=0
WARN=0
ok()   { printf '  \033[32m✓\033[0m %s\n' "$*"; }
warn() { printf '  \033[33m!\033[0m %s\n' "$*"; WARN=$((WARN+1)); }
bad()  { printf '  \033[31m✗\033[0m %s\n' "$*"; FAIL=$((FAIL+1)); }

DOMAIN="${1:-}"
if [ -z "$DOMAIN" ] && [ -f .env ]; then
  DOMAIN=$(grep -E '^ALTIUS_DOMAIN=' .env | head -1 | cut -d= -f2-)
fi

echo "Altius FTM — deployment preflight"
echo

# --- toolchain -------------------------------------------------------------
echo "Toolchain"
if command -v docker >/dev/null 2>&1; then
  ok "docker $(docker --version | awk '{print $3}' | tr -d ,)"
  docker info >/dev/null 2>&1 && ok "docker daemon running" || bad "docker daemon is not running"
else
  bad "docker not installed"
fi

if docker compose version >/dev/null 2>&1; then
  CV=$(docker compose version --short 2>/dev/null | sed 's/^v//')
  # The overlay uses the `!override` YAML tag, added in Compose v2.24. Older
  # versions MERGE those keys instead, which silently leaves `caddy` depending
  # on the web/landing services that this topology never starts.
  if [ -n "$CV" ] && [ "$(printf '%s\n2.24.0\n' "$CV" | sort -V | head -1)" = "2.24.0" ]; then
    ok "docker compose v$CV (>= 2.24, supports !override)"
  else
    bad "docker compose v${CV:-unknown} is too old — the overlay needs >= 2.24 for !override"
  fi
else
  bad "docker compose v2 plugin not available"
fi
echo

# --- resources -------------------------------------------------------------
echo "Resources"
if [ -r /proc/meminfo ]; then
  MEM_MB=$(( $(awk '/MemTotal/{print $2}' /proc/meminfo) / 1024 ))
  SWAP_MB=$(( $(awk '/SwapTotal/{print $2}' /proc/meminfo) / 1024 ))
  TOTAL=$((MEM_MB + SWAP_MB))
  if [ "$TOTAL" -ge 6000 ]; then
    ok "memory ${MEM_MB}MB + swap ${SWAP_MB}MB = ${TOTAL}MB"
  else
    bad "only ${TOTAL}MB of RAM+swap — the Rust release build is likely to be OOM-killed"
    echo "      add swap:  fallocate -l 4G /swapfile && chmod 600 /swapfile \\"
    echo "                 && mkswap /swapfile && swapon /swapfile"
    echo "      persist :  echo '/swapfile none swap sw 0 0' >> /etc/fstab"
  fi
else
  warn "cannot read /proc/meminfo (not Linux?) — check RAM+swap >= 6GB manually"
fi

AVAIL_GB=$(df -Pk . | awk 'NR==2{print int($4/1024/1024)}')
if [ "${AVAIL_GB:-0}" -ge 15 ]; then ok "disk ${AVAIL_GB}GB free"
else bad "only ${AVAIL_GB}GB free — images plus the cargo build cache need ~15GB"; fi
echo

# --- network ---------------------------------------------------------------
echo "Network"
if [ -z "$DOMAIN" ] || [ "$DOMAIN" = "localhost" ]; then
  warn "ALTIUS_DOMAIN not set — Caddy will serve plain HTTP on :80 only, no TLS"
else
  RESOLVED=$(getent hosts "$DOMAIN" 2>/dev/null | awk '{print $1}' | head -1)
  [ -z "$RESOLVED" ] && RESOLVED=$(dig +short "$DOMAIN" A 2>/dev/null | tail -1)
  PUBIP=$(curl -fsS --max-time 8 https://api.ipify.org 2>/dev/null \
          || curl -fsS --max-time 8 https://ifconfig.me 2>/dev/null || echo "")

  if [ -z "$RESOLVED" ]; then
    bad "$DOMAIN does not resolve — create an A record before starting, or Caddy cannot pass the ACME challenge"
  elif [ -n "$PUBIP" ] && [ "$RESOLVED" != "$PUBIP" ]; then
    bad "$DOMAIN resolves to $RESOLVED but this host is $PUBIP — ACME will fail"
  elif [ -n "$PUBIP" ]; then
    ok "$DOMAIN -> $RESOLVED (matches this host)"
  else
    warn "$DOMAIN -> $RESOLVED (could not determine this host's public IP to compare)"
  fi
fi

for p in 80 443; do
  # `ss` on a modern server, `lsof` as the portable fallback.
  if command -v ss >/dev/null 2>&1; then
    BUSY=$(ss -lntH "sport = :$p" 2>/dev/null | head -1)
  else
    BUSY=$(lsof -nP -iTCP:$p -sTCP:LISTEN 2>/dev/null | sed -n 2p)
  fi
  # Caddy holding the port across a redeploy is expected, not a conflict.
  if [ -n "$BUSY" ] && ! docker ps --format '{{.Ports}}' 2>/dev/null | grep -q ":$p->"; then
    bad "port $p already in use by something other than this stack"
  else
    ok "port $p available (or held by this stack)"
  fi
done
echo

# --- configuration ---------------------------------------------------------
echo "Configuration"
if [ -f .env ]; then
  ok ".env present"
  get() { grep -E "^$1=" .env | head -1 | cut -d= -f2-; }

  # Compose does NOT derive DATABASE_URL from POSTGRES_PASSWORD; a mismatch
  # crash-loops the API with an authentication failure.
  PGPW=$(get POSTGRES_PASSWORD); DBURL=$(get DATABASE_URL)
  if [ -n "$PGPW" ] && [ -n "$DBURL" ]; then
    case "$DBURL" in
      *":${PGPW}@"*) ok "DATABASE_URL password matches POSTGRES_PASSWORD" ;;
      *) bad "DATABASE_URL password does not match POSTGRES_PASSWORD — the API will crash-loop" ;;
    esac
  fi

  case "$DBURL" in
    *@localhost:*|*@127.0.0.1:*)
      bad "DATABASE_URL points at localhost — inside the api container that is the container itself, use @postgres:5432" ;;
  esac

  # Unset is fine wherever compose already defaults to the right value — only
  # an explicitly WRONG value is a problem. Flagging a correct-by-default blank
  # would make this script cry wolf, and a preflight nobody trusts is useless.
  AUD=$(get KEYCLOAK_AUDIENCE)
  case "$AUD" in
    ""|altius-api) ok "KEYCLOAK_AUDIENCE=${AUD:-altius-api (compose default)}" ;;
    *) bad "KEYCLOAK_AUDIENCE='${AUD}' — the realm mints aud=altius-api, so every token would be rejected" ;;
  esac

  # Keycloak listens on 8081. 8080 is the API's port.
  for k in KEYCLOAK_JWKS_URL KEYCLOAK_TOKEN_URL; do
    v=$(get "$k")
    case "$v" in
      *:8080/*) bad "$k points at port 8080 — Keycloak listens on 8081" ;;
      "") ok "$k unset (compose defaults to keycloak:8081)" ;;
      *) ok "$k port looks right" ;;
    esac
  done

  # Unlike the above, compose's default here (localhost:8081) is wrong for any
  # public host, so an unset value is worth calling out.
  ISS=$(get KEYCLOAK_ISSUER)
  if [ -z "$ISS" ]; then
    warn "KEYCLOAK_ISSUER unset — compose defaults to localhost, wrong for a public host"
  elif case "$ISS" in *example.com*) true ;; *) false ;; esac; then
    bad "KEYCLOAK_ISSUER is still a placeholder ($ISS)"
  elif [ -z "$DOMAIN" ]; then
    ok "KEYCLOAK_ISSUER=$ISS (no ALTIUS_DOMAIN to compare against)"
  else
    case "$ISS" in
      *"$DOMAIN"*) ok "KEYCLOAK_ISSUER matches ALTIUS_DOMAIN" ;;
      *) bad "KEYCLOAK_ISSUER ($ISS) does not contain $DOMAIN — tokens will fail iss validation" ;;
    esac
  fi

  # Secrets that must not ship as the template value.
  for k in POSTGRES_PASSWORD KEYCLOAK_ADMIN_PASSWORD AGENT_STATE_SECRET; do
    case "$(get $k)" in
      ""|CHANGE-ME*) bad "$k is unset or still CHANGE-ME" ;;
      *) ok "$k set" ;;
    esac
  done

  case "$(get DEFAULT_ADMIN_SUB)" in
    CHANGE-ME*) warn "DEFAULT_ADMIN_SUB is a placeholder — expected before the first boot; set it to the admin's Keycloak UUID afterwards or they get 403 everywhere" ;;
    "") warn "DEFAULT_ADMIN_SUB unset" ;;
    *) ok "DEFAULT_ADMIN_SUB set" ;;
  esac

  case "$(get CORS_ORIGINS)" in
    "") warn "CORS_ORIGINS empty — denies every cross-origin call; set it once the dashboard origin is known (deploy/set-web-origin.sh)" ;;
    *) ok "CORS_ORIGINS set" ;;
  esac
else
  bad ".env missing — run deploy/bootstrap-env.sh <domain> [dashboard-origin]"
fi

echo
if [ "$FAIL" -gt 0 ]; then
  echo "BLOCKED: $FAIL problem(s) to fix before deploying, $WARN warning(s)."
  exit 1
elif [ "$WARN" -gt 0 ]; then
  echo "OK with $WARN warning(s) — review them, then deploy."
  exit 2
else
  echo "All clear. Deploy with:"
  echo "  docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d --build --wait"
  exit 0
fi
