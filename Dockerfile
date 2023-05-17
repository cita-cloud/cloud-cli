FROM rust:slim-bullseye AS builder
WORKDIR /build
ENV PROTOC_NO_VENDOR 1
RUN rustup component add rustfmt && \
    apt-get update && \
    apt-get install -y --no-install-recommends make wget librocksdb-dev libsnappy-dev liblz4-dev libzstd-dev libssl-dev pkg-config clang protobuf-compiler && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*
COPY . /build
RUN cargo build --release

FROM debian:bullseye-slim
RUN useradd -m chain
USER chain
COPY --from=builder /build/target/release/cldi /usr/bin/
ENTRYPOINT ["cldi"]
