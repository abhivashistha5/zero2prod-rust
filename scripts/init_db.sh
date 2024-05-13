#!/usr/bin/env bash

# set -x
# set -eo pipefail

# check if commands are installed
POSTGRES_CLI="";

if [ -x "$(command -v psql)" ]; then
  POSTGRES_CLI="psql"
fi


if [ -x "$(command -v pgcli)" ]; then
  POSTGRES_CLI="pgcli";
fi

if [[ -z $POSTGRES_CLI ]]; then
  echo >&2 "Postgres cli not found"
  exit 1
fi

if ! [ -x "$(command -v sqlx)" ]; then
  echo >&2 "sqlx not installed."
  echo >&2 "Use:"
  echo >&2 "cargo install sqlx-cli --no-default-features --features rustls,postgres"
  echo >&2 "to install it."
  exit 1
fi

# Set env variables
DB_USER="${POSTGRES_USER:=postgres}"
DB_PASSWORD="${POSTGRES_PASSWORD:=password}"
DB_NAME="${POSTGRES_DB:=newsletter}"
DB_PORT="${POSTGRES_PORT:=5432}"
DB_HOST="${POSTGRES_HOST:=localhost}"
export PGPASSWORD="${DB_PASSWORD}"
export DATABASE_URL="postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}"

# Launch docker
if [[ -z $SKIP_DOCKER ]]; then
  docker run \
    -e POSTGRES_USER=${DB_USER} \
    -e POSTGRES_PASSWORD=${DB_PASSWORD} \
    -e POSTGRES_DB="${DB_NAME}" \
    -p "${DB_PORT}:5432" \
    -d --name newsletter_db \
    postgres -N 1000
fi

# Check for postgres ready
until ${POSTGRES_CLI} -h "${DB_HOST}" -U "${DB_USER}" -p "${DB_PORT}" -l &> /dev/null; do
  >&2 echo "Postgres is still unavailable - sleeping"
  sleep 1
done

>&2 echo "Postgres is up and running on port ${DB_PORT}"

sqlx database create
sqlx migrate run

>&2 echo "Postgres has been migrated, ready to go!"

