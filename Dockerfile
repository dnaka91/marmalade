# syntax = docker/dockerfile:1.2
FROM clux/muslrust:stable as builder

WORKDIR /volume

COPY assets/ assets/
COPY src/ src/
COPY templates/ templates/
COPY Cargo.lock Cargo.toml ./

RUN --mount=type=cache,target=/root/.cargo/git \
    --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/volume/target \
    cargo install --locked --path .

RUN strip --strip-all /root/.cargo/bin/marmalade

FROM alpine:3.14 as git

WORKDIR /volume

RUN apk add --no-cache autoconf curl gcc make musl-dev tar zlib-dev zlib-static \
    && curl -Lo- https://git.kernel.org/pub/scm/git/git.git/snapshot/git-2.33.0.tar.gz | tar -xz --strip-components=1 \
    && make configure \
    && ./configure CFLAGS="-O2 -static" LDFLAGS="-s" --without-openssl --without-libpcre2 --without-curl --without-expat --without-tcltk \
    && make -j $(nproc) git-receive-pack git-upload-pack

FROM scratch

COPY --from=builder /root/.cargo/bin/marmalade /bin/
COPY --from=git /volume/git-receive-pack /volume/git-upload-pack /bin/

EXPOSE 8080
STOPSIGNAL SIGINT

ENTRYPOINT ["/bin/marmalade"]
