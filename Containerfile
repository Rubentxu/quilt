# =============================================================================
# Quilt — Multi-stage container image
# Build: podman build -t quilt:latest -f Containerfile .
# Run:   podman run -d -p 3737:3737 -v quilt-data:/data quilt:latest
# Test:  podman build --target test -t quilt:test -f Containerfile .
# =============================================================================

# ── Stage 1: Rust builder ──────────────────────────────────
FROM docker.io/library/rust:1.89-slim AS rust-builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

RUN cargo build --release -p quilt-server \
    && cp target/release/quilt-server /usr/local/bin/

# ── Stage 2: Node.js builder (React frontend) ───────────────
FROM docker.io/library/node:22-slim AS node-builder

WORKDIR /src
COPY quilt-ui/package.json quilt-ui/package-lock.json ./
RUN npm ci

COPY quilt-ui/ ./
RUN npm run build

# ── Stage 3: Test runner (all testing layers) ───────────────
FROM docker.io/library/rust:1.89-slim AS test

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev libsqlite3-dev \
    nodejs npm \
    libnss3 libnspr4 libatk-bridge2.0-0 libdrm2 libxkbcommon0 \
    libxcomposite1 libxdamage1 libxfixes3 libxrandr2 \
    libgbm1 libpango-1.0-0 libcairo2 libasound2t64 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY package.json package-lock.json playwright.config.ts ./
COPY quilt-ui/package.json quilt-ui/package-lock.json quilt-ui/
COPY tests/ tests/
COPY justfile ./

RUN cd quilt-ui && npm ci && cd .. && npm ci \
    && npx playwright install chromium --with-deps
RUN cargo build --tests --workspace 2>/dev/null || true

CMD ["cargo", "test"]

# ── Stage 4: Production runtime ─────────────────────────────
FROM docker.io/library/debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libsqlite3-0 curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --shell /bin/bash appuser
WORKDIR /home/appuser
RUN mkdir -p /home/appuser/.quilt-data && chown -R appuser:appuser /home/appuser

COPY --from=rust-builder /usr/local/bin/quilt-server /usr/local/bin/
COPY --from=node-builder /src/dist /usr/local/share/quilt/frontend/

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3737/health || exit 1

USER appuser
EXPOSE 3737
ENV QUILT_GRAPH_DIR=/home/appuser/.quilt-data
ENV QUILT_FRONTEND_DIR=/usr/local/share/quilt/frontend
CMD ["quilt-server"]
