FROM rust:1.57 AS builder
RUN rustup component add rustfmt
WORKDIR /build
COPY . /build
RUN cargo build --release
FROM debian:buster
COPY --from=builder /build/target/release/cldi /usr/bin/
CMD ["cldi"]
