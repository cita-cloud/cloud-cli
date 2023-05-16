FROM rust:slim-bullseye AS builder
WORKDIR /build
ENV PROTOC_NO_VENDOR 1
RUN rustup component add rustfmt && \
    apt-get update && \
    apt-get install -y protobuf-compiler pkg-config libssl-dev && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*
COPY . /build
RUN cargo build --release

FROM debian:bullseye-slim
RUN useradd -m chain
USER chain
COPY --from=builder /build/target/release/cldi /usr/bin/
ENTRYPOINT ["cldi"]
