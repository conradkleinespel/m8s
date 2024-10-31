FROM rust:alpine AS builder
COPY . /app
WORKDIR /app
RUN apk add musl-dev
RUN cargo build --release

FROM alpine
COPY --from=builder /app/target/release/m8s /usr/bin/m8s
ENTRYPOINT ["/usr/bin/m8s"]
