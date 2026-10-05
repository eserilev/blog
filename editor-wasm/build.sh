#!/usr/bin/env bash
# Builds the editor preview (spec 6.7) into static/wasm/logbook_render.wasm.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -q -p logbook-editor-wasm --target wasm32-unknown-unknown --profile wasm
mkdir -p static/wasm
cp target/wasm32-unknown-unknown/wasm/logbook_editor_wasm.wasm static/wasm/logbook_render.wasm
echo "built static/wasm/logbook_render.wasm ($(wc -c < static/wasm/logbook_render.wasm) bytes)"
