# Copyright: Cardly contributors
# License: GNU AGPL, version 3 or later; https://www.gnu.org/licenses/agpl-3.0.html

# ============================================================================
# Stage 1: Build upstream Anki web assets + protobuf descriptors + Cardly server
# ============================================================================
FROM rust:1.86-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    bash \
    ca-certificates \
    curl \
    g++ \
    gcc \
    git \
    make \
    ninja-build \
    pkg-config \
    protobuf-compiler \
    rsync \
    xz-utils \
    && rm -rf /var/lib/apt/lists/*

# Build inside a space-free path so upstream ./ninja works unmodified.
WORKDIR /build

# Copy upstream Anki source (excluding any local out/ artifacts)
COPY anki-upstream ./anki-upstream

# Build upstream proto descriptors, FTL strings, and SvelteKit + reviewer web bundle
WORKDIR /build/anki-upstream
ENV PROTOC_BINARY=/usr/bin/protoc
RUN rm -rf .git && git init && git config user.email "build@cardly.local" && git config user.name "Cardly" && git add -A && git commit -m "init"
RUN ./ninja sveltekit qt:aqt:data:web rslib

# Copy Cardly Rust server crate and build release binary
WORKDIR /build
COPY server ./server
WORKDIR /build/server
ENV DESCRIPTORS_BIN=/build/anki-upstream/out/rslib/proto/descriptors.bin
ENV PROTOC=/usr/bin/protoc
ENV STRINGS_PY=/build/anki-upstream/out/pylib/anki/_fluent.py
ENV STRINGS_TS=/build/anki-upstream/out/ts/lib/generated/ftl.ts
RUN cargo build --release --locked || cargo build --release

# ============================================================================
# Stage 2: Build Cardly Svelte 5 Web Shell (`web/`)
# ============================================================================
FROM node:22-bookworm-slim AS web-builder

WORKDIR /build
COPY --from=builder /build/anki-upstream/out/ts/lib/generated ./anki-upstream/out/ts/lib/generated
COPY --from=builder /build/anki-upstream/out/node_modules/@bufbuild ./anki-upstream/out/node_modules/@bufbuild
COPY web ./web

WORKDIR /build/web
RUN npm ci || npm install
RUN npm run build

# ============================================================================
# Stage 3: Minimal runtime image
# ============================================================================
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --shell /usr/sbin/nologin cardly

WORKDIR /app
COPY --from=builder /build/server/target/release/cardly-server /app/cardly-server
COPY --from=builder /build/anki-upstream/out/qt/_aqt/data/web /app/web-upstream
COPY --from=web-builder /build/web/dist /app/web-shell
COPY LICENSE /app/LICENSE

RUN mkdir -p /data && chown -R cardly:cardly /app /data

USER cardly

ENV CARDLY_HOST=0.0.0.0
ENV CARDLY_PORT=8080
ENV CARDLY_DATA_DIR=/data
ENV CARDLY_UPSTREAM_WEB_DIR=/app/web-upstream
ENV CARDLY_SHELL_DIR=/app/web-shell
ENV RUST_LOG=cardly_server=info,anki=info

EXPOSE 8080

VOLUME ["/data"]

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["/app/cardly-server", "--healthcheck"]

ENTRYPOINT ["/app/cardly-server"]
