#!/usr/bin/env bash
# Starts the server for the browser tests: a fresh database with sample posts.
set -euo pipefail
cd "$(dirname "$0")/.."
export LOGBOOK_DB="${LOGBOOK_DB:-$PWD/e2e/.tmp/e2e.db}"
export LOGBOOK_ADDR=127.0.0.1:18100
export LOGBOOK_ORIGIN=http://localhost:18100
export LOGBOOK_SURF=off
# Many tests sign in within one minute. The rate limit has its own Rust tests.
export LOGBOOK_AUTH_RATE_LIMIT=1000
mkdir -p "$(dirname "$LOGBOOK_DB")"
rm -f "$LOGBOOK_DB" "$LOGBOOK_DB"-*
cargo build -q -p logbook-server
editor-wasm/build.sh
./target/debug/logbook seed-sample
exec ./target/debug/logbook serve
