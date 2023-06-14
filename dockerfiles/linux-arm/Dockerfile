FROM ghcr.io/cross-rs/aarch64-unknown-linux-gnu:latest
RUN dpkg --add-architecture arm64 && \
    apt-get update && \
    apt-get install --assume-yes libssl-dev:arm64 &&  \
    apt-get install --assume-yes --no-install-recommends make wget librocksdb-dev libsnappy-dev liblz4-dev libzstd-dev libssl-dev pkg-config clang protobuf-compiler
