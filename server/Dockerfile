FROM rust:1.80-bullseye as builder

WORKDIR /usr/src/app
COPY Cargo.toml Cargo.lock* ./
# Create dummy main to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

COPY src ./src
RUN touch src/main.rs && cargo build --release

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /usr/src/app
COPY --from=builder /usr/src/app/target/release/secure-chat-server /usr/local/bin/secure-chat-server
COPY .env.example .env

EXPOSE 5000
CMD ["secure-chat-server"]
