# syntax = docker/dockerfile:1.2
FROM clux/muslrust:stable as builder

WORKDIR /volume

COPY assets/ assets/
COPY src/ src/
COPY templates/ templates/
COPY build.rs Cargo.lock Cargo.toml ./

RUN --mount=type=cache,target=/root/.cargo/git \
    --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/volume/target \
    cargo install --locked --path .

RUN strip --strip-all /root/.cargo/bin/marmalade

FROM alpine:3.14

RUN apk add --no-cache git=2.32.0-r0 \
    && addgroup -g 1000 marmalade \
    && adduser -u 1000 -G marmalade -D -g '' -H -h /dev/null -s /sbin/nologin marmalade

COPY --from=builder /root/.cargo/bin/marmalade /bin/

EXPOSE 8080
STOPSIGNAL SIGINT
USER marmalade

ENTRYPOINT ["/bin/marmalade"]
