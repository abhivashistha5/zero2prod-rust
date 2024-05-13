FROM lukemathwalker/cargo-chef:latest-rust-slim-bullseye as chef
WORKDIR /app
RUN apt-get update && apt-get install lld clang pkg-config openssl ca-certificates -y

FROM chef as planner
COPY . .

# Compute a lock file for our project
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Build project Dependencies not our application
RUN cargo chef cook --release --recipe-path recipe.json

# UPTO this point if Dependencies are not changed then everything is cahced

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
