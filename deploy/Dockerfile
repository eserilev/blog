# syntax=docker/dockerfile:1
# The app image: the logbook binary, Litestream, and the static files (spec 6.11).
# Build from the repo root: docker build -f deploy/Dockerfile .

FROM rust:1.97.1-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml clippy.toml ./
COPY crates/ crates/
COPY editor-wasm/ editor-wasm/
RUN rustup target add wasm32-unknown-unknown
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p logbook-server \
    && cargo build --locked -p logbook-editor-wasm --target wasm32-unknown-unknown --profile wasm \
    && cp target/release/logbook /logbook \
    && cp target/wasm32-unknown-unknown/wasm/logbook_editor_wasm.wasm /logbook_render.wasm

FROM debian:12-slim AS litestream
ARG LITESTREAM_VERSION=0.5.17
ARG LITESTREAM_SHA256=cfb371176d164437ae869f8351cfde49bd1804ae71c61923f75c9cba9c9c006d
ADD https://github.com/benbjohnson/litestream/releases/download/v${LITESTREAM_VERSION}/litestream-${LITESTREAM_VERSION}-linux-x86_64.tar.gz /litestream.tar.gz
RUN echo "${LITESTREAM_SHA256}  /litestream.tar.gz" | sha256sum -c - \
    && tar -xzf /litestream.tar.gz -C / litestream

FROM debian:12-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates sqlite3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 app \
    && mkdir -p /data && chown app:app /data
COPY --from=build /logbook /usr/local/bin/logbook
COPY --from=litestream /litestream /usr/local/bin/litestream
COPY static/ /app/static/
COPY --from=build /logbook_render.wasm /app/static/wasm/logbook_render.wasm
COPY deploy/litestream.yml /etc/litestream.yml
COPY --chmod=0755 deploy/entrypoint.sh /usr/local/bin/entrypoint
USER app
WORKDIR /app
ENV LOGBOOK_ADDR=0.0.0.0:8080 \
    LOGBOOK_STATIC_DIR=/app/static \
    LOGBOOK_DB=/data/logbook.db
VOLUME /data
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/entrypoint"]
