FROM rust:1.59 AS builder
WORKDIR /build
ENV PROTOC_NO_VENDOR 1
RUN rustup component add rustfmt && \
    apt-get update && \
    apt-get install -y protobuf-compiler && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*
COPY . /build
RUN cargo build --release

FROM debian:buster
RUN useradd -m chain
USER chain
COPY --from=builder /build/target/release/cldi /usr/bin/
ENTRYPOINT ["cldi"]
