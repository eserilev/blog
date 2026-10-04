#!/bin/sh
# Container start (spec 6.12):
#   1. Restore the database from the bucket if there is no local copy.
#   2. No replica and no local copy: start empty only with ALLOW_EMPTY_START=1.
#   3. RESTORE_ONLY=1: run without replication (drills).
#   4. Else: replicate, with the app as the child process.
set -eu

: "${LOGBOOK_DB:=/data/logbook.db}"
: "${S3_BUCKET:?set S3_BUCKET}"
: "${S3_ENDPOINT:?set S3_ENDPOINT}"
: "${S3_REGION:=auto}"
: "${S3_FORCE_PATH_STYLE:=false}"
export LOGBOOK_DB S3_REGION S3_FORCE_PATH_STYLE

CONFIG=/etc/litestream.yml

# An unreachable bucket or bad credentials make this fail, and the container stops.
litestream restore -config "$CONFIG" -if-db-not-exists -if-replica-exists "$LOGBOOK_DB"

if [ ! -f "$LOGBOOK_DB" ]; then
  if [ "${ALLOW_EMPTY_START:-}" = "1" ]; then
    echo "entrypoint: no replica in s3://${S3_BUCKET}/db; ALLOW_EMPTY_START=1, starting with a new database"
  else
    echo "entrypoint: no replica found in s3://${S3_BUCKET}/db; refusing to start empty." >&2
    echo "entrypoint: set ALLOW_EMPTY_START=1 for the first deploy only." >&2
    exit 1
  fi
fi

if [ "${RESTORE_ONLY:-}" = "1" ]; then
  echo "entrypoint: RESTORE_ONLY=1, running without replication; local writes are discarded"
  exec logbook serve
fi

exec litestream replicate -config "$CONFIG" -exec "logbook serve"
