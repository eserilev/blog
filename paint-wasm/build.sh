#!/usr/bin/env bash
# Builds Paint (spec 11.9) into static/wasm/logbook_paint.wasm.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -q -p logbook-paint-wasm --target wasm32-unknown-unknown --profile wasm
mkdir -p static/wasm
cp "${CARGO_TARGET_DIR:-target}"/wasm32-unknown-unknown/wasm/logbook_paint_wasm.wasm static/wasm/logbook_paint.wasm
echo "built static/wasm/logbook_paint.wasm ($(wc -c < static/wasm/logbook_paint.wasm) bytes)"
