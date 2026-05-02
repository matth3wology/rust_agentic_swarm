FROM rust:1.87-slim-bookworm AS builder
WORKDIR /app

# Build dependency layer first for better cache reuse.
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release || true

# Build the real application.
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/ai-agent /usr/local/bin/ai-agent

# Keep the runtime non-root for better cluster security defaults.
RUN useradd --create-home --uid 10001 appuser
USER 10001

ENTRYPOINT ["/usr/local/bin/ai-agent"]
