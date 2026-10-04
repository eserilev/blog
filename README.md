# Eitan's Logbook

A personal blog with a 1998 desktop look. Rust (axum, SQLite) and native web components.

- Spec: [`spec.md`](spec.md)
- Design mockup: [`designs/logbook.html`](designs/logbook.html)

## Run locally

```bash
cargo run -p logbook-server
# open http://127.0.0.1:8080
```

Settings: `LOGBOOK_ADDR` (default `127.0.0.1:8080`), `LOGBOOK_STATIC_DIR` (default `static`).

## Checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check
cargo nextest run --workspace
```
