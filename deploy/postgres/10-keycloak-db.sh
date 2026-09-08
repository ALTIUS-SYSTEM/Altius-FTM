#!/bin/sh
# Creates Keycloak's own database inside the same Postgres instance.
#
# Keycloak and the Altius API both need durable storage, but they must not
# share a schema: Keycloak manages its ~90 tables with its own Liquibase
# migrations, the API with refinery. One database per owner keeps those two
# migration histories from ever meeting.
#
# Runs only on FIRST initialisation of the data directory (postgres image
# convention). On a host whose volume already exists, create it by hand:
#   docker compose exec postgres createdb -U "$POSTGRES_USER" keycloak
set -eu

DB="${KC_POSTGRES_DB:-keycloak}"

# createdb would fail the whole entrypoint if the database already exists, so
# ask first — the init dir can be re-run against a restored volume.
if psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" \
     -tAc "SELECT 1 FROM pg_database WHERE datname = '$DB'" | grep -q 1; then
  echo "database '$DB' already present, skipping"
else
  createdb --username "$POSTGRES_USER" "$DB"
  echo "created database '$DB' for Keycloak"
fi
