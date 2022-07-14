FROM rust:1.59 AS builder
RUN rustup component add rustfmt
WORKDIR /build
ENV PROTOC_NO_VENDOR 1
COPY . /build
RUN cargo build --release\
    apt-get update;\
    apt-get install -y protobuf-compiler;
FROM debian:buster
COPY --from=builder /build/target/release/cldi /usr/bin/
ENTRYPOINT ["cldi"]
