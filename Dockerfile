ARG RUST_VERSION=1.95
ARG DEBIAN_VERSION=13
ARG RUN_TAG=nonroot
ARG BUILD_BASE=rust:${RUST_VERSION}-slim
ARG RUN_BASE=gcr.io/distroless/cc-debian${DEBIAN_VERSION}:${RUN_TAG}

FROM ${BUILD_BASE} AS builder

WORKDIR /src

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --locked

FROM ${RUN_BASE} AS runner

COPY --from=builder /src/target/release/github-actions-exporter /usr/local/bin/github-actions-exporter

USER 65532:65532

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/github-actions-exporter"]
