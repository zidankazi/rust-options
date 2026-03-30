# Stage 1: Build frontend
FROM node:20-slim AS frontend
WORKDIR /app/crates/web/frontend
COPY crates/web/frontend/package.json crates/web/frontend/package-lock.json ./
RUN npm ci
COPY crates/web/frontend/ ./
RUN npm run build

# Stage 2: Build Rust backend
FROM rust:1-slim AS backend
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY crates/pricer/ crates/pricer/
COPY crates/market-data/ crates/market-data/
COPY crates/risk/ crates/risk/
COPY crates/strategy/ crates/strategy/
COPY crates/web/Cargo.toml crates/web/Cargo.toml
COPY crates/web/src/ crates/web/src/
# Need a stub lib.rs for workspace to compile
RUN mkdir -p crates/web/static
COPY --from=frontend /app/crates/web/static crates/web/static/
RUN cargo build --release -p web

# Stage 3: Minimal runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=backend /app/target/release/server ./server
COPY --from=backend /app/crates/web/static ./crates/web/static/
ENV PORT=3000
EXPOSE 3000
CMD ["./server"]
