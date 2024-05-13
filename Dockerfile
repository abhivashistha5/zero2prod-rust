# Builde stage
FROM rust:1.78.0 AS builder

WORKDIR /app

# Install System Dependencies
RUN apt update && apt install lld clang -y

COPY . .

ENV SQLX_OFFLINE true
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim AS runtime

WORKDIR /app

# Install runtime Dependencies
RUN apt-get update -y \
  && apt-get install -y --no-install-recommends openssl ca-certificates \
  && apt-get autoremove -y \
  && apt-get clean -y \
  && rm -rf /var/lib/apt/lists/*

# Copy binary from builder to runtime image
COPY --from=builder /app/target/release/zero2prod-rust zero2prod-rust

# Copy configuration
COPY configuration configuration
ENV APP_ENV production
ENTRYPOINT ["./zero2prod-rust"]
