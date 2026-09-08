#!/usr/bin/env bash
# Point the deployment at the dashboard's browser origin.
#
#   ./deploy/set-web-origin.sh https://altius-ftm.vercel.app
#   ./deploy/set-web-origin.sh https://app.altiussystem.com https://staging.altiussystem.com
#
# Run this whenever the dashboard's origin is first known, or later changes.
# The realm import (deploy/keycloak-prod) only seeds these values on the very
# first boot; from then on the client is edited in place, which is what this
# does. Nothing here recreates the realm or touches user data.
#
# Three things have to agree or login breaks in a way that looks like a bug:
#   1. Keycloak `altius-web`  — redirect URIs / web origins / post-logout
#   2. The API's CORS_ORIGINS — empty or wrong denies every browser call
#   3. The API's KEYCLOAK_REDIRECT_URI
# This script sets all three and restarts the API. Safe to re-run.
set -euo pipefail

cd "$(dirname "$0")/.."

if [ $# -lt 1 ]; then
  cat >&2 <<'USAGE'
usage: deploy/set-web-origin.sh <origin> [more origins...]

  <origin>  full browser origin, scheme included, no trailing path
            e.g. https://altius-ftm.vercel.app

Pass several to allow preview deployments as well as production.
USAGE
  exit 64
fi

ORIGINS=()
for o in "$@"; do
  case "$o" in
    https://*|http://*) ;;
    *) echo "error: origin must start with https:// or http:// — got '$o'" >&2; exit 64 ;;
  esac
  # Strip a trailing slash BEFORE the path check — otherwise a perfectly good
  # "https://host/" is rejected as having a path. Left in, it would also yield
  # "https://host//callback" in the realm, which Keycloak will not match.
  o="${o%/}"
  case "${o#*://}" in
    */*) echo "error: origin must have no path — got '$o'" >&2; exit 64 ;;
    "")  echo "error: origin has no host — got '$o'" >&2; exit 64 ;;
  esac
  ORIGINS+=("$o")
done
PRIMARY="${ORIGINS[0]}"

[ -f .env ] || { echo "error: no .env here — run deploy/bootstrap-env.sh first" >&2; exit 1; }

# `docker compose -f a -f b` for the VPS topology; override for a plain stack.
read -r -a COMPOSE <<< "${ALTIUS_COMPOSE:-docker compose -f docker-compose.yml -f docker-compose.prod.yml}"

# --- 1. Keycloak client ----------------------------------------------------
# Build the JSON arrays kcadm expects.
redirects=$(printf '"%s/callback",' "${ORIGINS[@]}"); redirects="[${redirects%,}]"
weborigins=$(printf '"%s",' "${ORIGINS[@]}");         weborigins="[${weborigins%,}]"
# Keycloak stores multi-valued attributes as "##"-separated, not JSON.
postlogout=$(printf '%s##' "${ORIGINS[@]}");          postlogout="${postlogout%##}"

echo "Updating Keycloak client altius-web…"
"${COMPOSE[@]}" exec -T keycloak sh -s <<KCEOF
set -e
kcadm=/opt/keycloak/bin/kcadm.sh
\$kcadm config credentials --server http://localhost:8081 --realm master \
  --user "\$KC_BOOTSTRAP_ADMIN_USERNAME" --password "\$KC_BOOTSTRAP_ADMIN_PASSWORD" >/dev/null
id=\$(\$kcadm get clients -r altius -q clientId=altius-web --fields id --format csv --noquotes | tail -1)
[ -n "\$id" ] || { echo "error: client altius-web not found in realm altius" >&2; exit 1; }
\$kcadm update "clients/\$id" -r altius \
  -s 'redirectUris=${redirects}' \
  -s 'webOrigins=${weborigins}' \
  -s 'attributes."post.logout.redirect.uris"=${postlogout}'
\$kcadm get "clients/\$id" -r altius --fields redirectUris,webOrigins
KCEOF

# --- 2 & 3. .env -----------------------------------------------------------
# Rewrite in place, preserving every other line and any comments.
set_env() {
  local key="$1" val="$2"
  if grep -qE "^${key}=" .env; then
    # `|` as the delimiter: the values are URLs and contain `/`.
    sed -i.bak -E "s|^${key}=.*|${key}=${val}|" .env && rm -f .env.bak
  else
    printf '%s=%s\n' "$key" "$val" >> .env
  fi
  echo "  ${key}=${val}"
}

echo "Updating .env…"
csv=$(IFS=,; echo "${ORIGINS[*]}")
set_env ALTIUS_WEB_ORIGIN "$PRIMARY"
set_env CORS_ORIGINS "$csv"
set_env KEYCLOAK_REDIRECT_URI "${PRIMARY}/callback"
set_env KEYCLOAK_LOGOUT_REDIRECT_URI "$PRIMARY"

# --- restart the API so it reads the new CORS + redirect values -------------
echo "Restarting api…"
"${COMPOSE[@]}" up -d --wait api

echo
echo "Done. The dashboard at ${PRIMARY} can now sign in."
echo "Remember: apps/web inlines NEXT_PUBLIC_* at BUILD time, so set"
echo "NEXT_PUBLIC_API_BASE and NEXT_PUBLIC_KEYCLOAK_URL in Vercel and redeploy."
