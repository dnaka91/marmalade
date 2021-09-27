FROM rust:1.55 as builder

WORKDIR /volume

RUN apt-get update \
    && apt-get install -y --no-install-recommends musl-tools=1.2.2-1 \
    && rustup target add x86_64-unknown-linux-musl

COPY assets/ assets/
COPY src/ src/
COPY templates/ templates/
COPY build.rs Cargo.lock Cargo.toml ./

RUN cargo build --release --target x86_64-unknown-linux-musl \
    && strip --strip-all target/x86_64-unknown-linux-musl/release/marmalade

FROM alpine:3.14

RUN apk add --no-cache git=2.32.0-r0 \
    && addgroup -g 1000 marmalade \
    && adduser -u 1000 -G marmalade -D -g '' -H -h /dev/null -s /sbin/nologin marmalade

COPY --from=builder /volume/target/x86_64-unknown-linux-musl/release/marmalade /bin/

EXPOSE 8080
STOPSIGNAL SIGINT
USER marmalade

ENTRYPOINT ["/bin/marmalade"]
