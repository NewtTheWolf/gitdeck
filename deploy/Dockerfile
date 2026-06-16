# Multi-stage build for the standalone gitdeck-server (gRPC + gRPC-Web).
# Build context = repo root:  docker build -f deploy/Dockerfile -t gitdeck-server .

# ---- builder ----
FROM rust:1-bookworm AS builder

# protoc is vendored & compiled from source by the `protobuf-src` build-dep,
# which needs a C++ toolchain + cmake.
RUN apt-get update && apt-get install -y --no-install-recommends \
        cmake g++ make \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# Build only the server binary in release mode.
RUN cargo build --release -p newt-todo-api --bin gitdeck-server

# ---- runtime ----
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --uid 10001 gitdeck

COPY --from=builder /app/target/release/gitdeck-server /usr/local/bin/gitdeck-server

USER gitdeck
WORKDIR /home/gitdeck

# Data dir for the sqlite db (mount a volume here to persist).
ENV GITDECK_DB=/home/gitdeck/data/gitdeck.db \
    GITDECK_BIND=0.0.0.0:50061
RUN mkdir -p /home/gitdeck/data

EXPOSE 50061

# GITHUB_TOKEN and (optional) GITDECK_API_KEY are passed at runtime.
ENTRYPOINT ["gitdeck-server"]
