FROM rust:1.69 as builder

WORKDIR /app

COPY Cargo.toml ./
COPY src        ./src

RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates libssl-dev libpq5 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/statustracker /app/statustracker

CMD ["./statustracker"]
