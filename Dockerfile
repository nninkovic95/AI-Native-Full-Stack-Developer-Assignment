# Build stage: compiles the Axum server natively and the Yew frontend to WASM.
FROM rust:1-bookworm AS build
WORKDIR /app

RUN rustup target add wasm32-unknown-unknown \
 && curl -fsSL https://github.com/trunk-rs/trunk/releases/latest/download/trunk-x86_64-unknown-linux-gnu.tar.gz \
    | tar -xz -C /usr/local/bin

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY migrations ./migrations

RUN cargo build --release -p server
RUN cd crates/web && trunk build --release

# Runtime stage: just the binary and the static frontend.
FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /app/target/release/server /app/server
COPY --from=build /app/dist /app/dist
ENV DIST_DIR=/app/dist \
    PORT=10000
EXPOSE 10000
CMD ["/app/server"]
