# syntax=docker/dockerfile:1
FROM --platform=$BUILDPLATFORM rust:alpine AS builder
RUN apk add --no-cache musl-dev

ARG TARGETPLATFORM
RUN case "$TARGETPLATFORM" in \
    linux/amd64) TRIPLE="x86_64-unknown-linux-musl" ;; \
    linux/arm64) TRIPLE="aarch64-unknown-linux-musl" ;; \
    *) echo "Unsupported: $TARGETPLATFORM"; exit 1 ;; \
    esac && echo "$TRIPLE" > /triple && rustup target add "$TRIPLE"

WORKDIR /app
COPY . .
RUN cargo build --release --features http --target $(cat /triple) && \
    cp target/$(cat /triple)/release/arch-opsd /arch-opsd

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /arch-opsd /usr/local/bin/arch-opsd
CMD ["arch-opsd", "stdio"]
