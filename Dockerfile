FROM rust:1.75 as builder

WORKDIR /volume

ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

RUN apt-get update && \
    apt-get install -y --no-install-recommends musl-tools && \
    rustup target add x86_64-unknown-linux-musl && \
    cargo init --bin

COPY assets/ assets/
COPY build.rs Cargo.lock Cargo.toml ./

RUN cargo build --release --target x86_64-unknown-linux-musl

COPY src/ src/
COPY templates/ templates/

RUN touch src/main.rs && cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:3

RUN apk add --no-cache git && \
    addgroup -g 1000 marmalade && \
    adduser -u 1000 -G marmalade -D -g '' -H -h /dev/null -s /sbin/nologin marmalade

COPY --from=builder /volume/target/x86_64-unknown-linux-musl/release/marmalade /bin/

EXPOSE 8080
USER marmalade

ENTRYPOINT ["/bin/marmalade"]
