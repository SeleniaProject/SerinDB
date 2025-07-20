# ---- Build Stage ----
FROM rust:latest-alpine AS builder
WORKDIR /app
RUN apk add --no-cache musl-dev pkgconfig
COPY . .
RUN cargo build --release --bin serindb

# ---- Runtime Stage ----
FROM alpine:latest
WORKDIR /usr/local/bin
COPY --from=builder /app/target/release/serindb ./serindb
ENTRYPOINT ["/usr/local/bin/serindb"]
CMD ["--help"] 