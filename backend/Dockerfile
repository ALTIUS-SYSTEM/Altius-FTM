# syntax=docker/dockerfile:1

# ---------- build ----------
# Pinned: `rust:1-bookworm` and `debian:bookworm-slim` float, so an image built
# today and one built next month are not the same artifact.
FROM rust:1.90-bookworm AS build
WORKDIR /src

# Dependency layer, keyed on the manifests alone. Copying `crates` before the
# build (as before) meant every source edit rebuilt the whole dependency graph.
COPY Cargo.toml Cargo.lock ./
COPY crates/altius-api/Cargo.toml crates/altius-api/
COPY crates/altius-core/Cargo.toml crates/altius-core/
COPY crates/altius-schema/Cargo.toml crates/altius-schema/
RUN mkdir -p crates/altius-api/src crates/altius-core/src crates/altius-schema/src \
 && echo 'fn main() {}' > crates/altius-api/src/main.rs \
 && : > crates/altius-core/src/lib.rs \
 && : > crates/altius-schema/src/lib.rs
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cargo-target,target=/src/target \
    cargo build --release --locked -p altius-api

# Real sources; touch so cargo discards the stub fingerprints.
COPY crates ./crates
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cargo-target,target=/src/target \
    touch crates/*/src/*.rs \
 && cargo build --release --locked -p altius-api \
 && cp target/release/altius-api /altius-api

# ---------- runtime ----------
FROM debian:bookworm-slim
# No curl: the binary self-probes via `--health-check` (see main.rs), which
# keeps the runtime image to ca-certificates alone.
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && groupadd --system altius \
 && useradd --system --gid altius --home-dir /nonexistent --shell /usr/sbin/nologin altius
COPY --from=build /altius-api /usr/local/bin/altius-api
ENV BIND=0.0.0.0:8080
USER altius
EXPOSE 8080
HEALTHCHECK --interval=15s --timeout=5s --retries=3 --start-period=30s \
  CMD ["/usr/local/bin/altius-api", "--health-check"]
ENTRYPOINT ["altius-api"]
