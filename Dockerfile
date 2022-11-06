FROM rust:1.65 as builder

WORKDIR /volume

RUN apt-get update && \
    apt-get install -y --no-install-recommends musl-tools=1.2.2-1 && \
    rustup target add x86_64-unknown-linux-musl && \
    cargo init --bin

COPY assets/ assets/
COPY build.rs Cargo.lock Cargo.toml ./

RUN cargo build --release --target x86_64-unknown-linux-musl

COPY src/ src/
COPY templates/ templates/

RUN touch src/main.rs && cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:3.16

RUN apk add --no-cache git=~2.36 && \
    addgroup -g 1000 marmalade && \
    adduser -u 1000 -G marmalade -D -g '' -H -h /dev/null -s /sbin/nologin marmalade

COPY --from=builder /volume/target/x86_64-unknown-linux-musl/release/marmalade /bin/

EXPOSE 8080
USER marmalade

ENTRYPOINT ["/bin/marmalade"]
