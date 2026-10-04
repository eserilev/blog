# Eitan's Logbook

A personal blog with a 1998 desktop look. Rust (axum, SQLite, Litestream) and native web components.

- Spec: [`spec.md`](spec.md)
- Design mockup: [`designs/logbook.html`](designs/logbook.html)

## Run locally

```bash
cargo run -p logbook-server -- seed-sample   # optional: sample posts in logbook.db
cargo run -p logbook-server                  # http://localhost:8080
cargo run -p logbook-server -- setup-link    # one-time link to register your passkey
```

Settings: `LOGBOOK_ADDR` (default `127.0.0.1:8080`), `LOGBOOK_STATIC_DIR` (default `static`),
`LOGBOOK_DB` (default `logbook.db`), `LOGBOOK_ORIGIN` (default `http://localhost:8080`; the host is the passkey RP ID, so not an IP),
`LOGBOOK_TRUSTED_PROXIES` (CIDRs, default none), `LOGBOOK_AUTH_RATE_LIMIT` (default 20 per minute).

## Checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check
cargo nextest run --workspace
deploy/restore-test.sh                                       # needs Docker
(cd e2e && npm ci && npx playwright install chromium && npx playwright test)
(cd fuzz && cargo +nightly fuzz run render -- -max_total_time=60)
```

## License

MIT. See [`LICENSE`](LICENSE). Fonts in `static/fonts/` use the SIL Open Font License.
