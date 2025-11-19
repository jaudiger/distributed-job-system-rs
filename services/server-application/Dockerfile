FROM docker.io/rust:1.91.1-slim-trixie AS base

FROM base AS dependencies
WORKDIR /builder
COPY Cargo.lock Cargo.toml ./
COPY services/client-application/Cargo.toml ./services/client-application/
COPY services/server-application/Cargo.toml ./services/server-application/
ENV CARGO_HOME=/home/container/.cargo
RUN set -eux; \
    mkdir -p services/client-application/src services/server-application/src; \
    echo 'fn main() {}' > services/client-application/src/main.rs; \
    echo 'fn main() {}' > services/server-application/src/main.rs; \
    cargo fetch --locked

FROM base AS builder
WORKDIR /builder
COPY --from=dependencies /home/container/.cargo /home/container/.cargo
COPY --from=dependencies /builder/Cargo.lock /builder/Cargo.toml ./
COPY --from=dependencies /builder/services/client-application/Cargo.toml ./services/client-application/
COPY --from=dependencies /builder/services/server-application/Cargo.toml ./services/server-application/
COPY services/client-application/src/ ./services/client-application/src/
COPY services/server-application/src/ ./services/server-application/src/
ENV CARGO_HOME=/home/container/.cargo
RUN set -eux; \
    apt-get update; \
    apt-get upgrade -y; \
    apt-get install -y --no-install-recommends clang=1:19.0-63 python3=3.13.5-1 make=4.4.1-2; \
    cargo build --locked --release --package server-application; \
    apt-get remove -y clang python3 make; \
    apt-get clean; \
    rm -rf /var/lib/apt/lists/*

FROM docker.io/debian:13.1-slim AS runtime
WORKDIR /app
RUN set -eux; \
    apt-get update; \
    apt-get upgrade -y; \
    apt-get install -y --no-install-recommends curl=8.14.1-2+deb13u2; \
    apt-get clean; \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /builder/target/release/server-application ./main
ENTRYPOINT [ "/app/main" ]
