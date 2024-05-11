FROM rust:1.78.0

WORKDIR /app

# Install System Dependencies
RUN apt update && apt install lld clang -y

COPY . .

ENV SQLX_OFFLINE true
RUN cargo build --release

ENV APP_ENV production
ENTRYPOINT ["./target/release/zero2prod-rust"]
