# ── Quilt development toolkit ──────────────────────────────────────────
# Install: cargo install just
# Usage:   just <command>

# ── Rust ──────────────────────────────────────────────────────────────

# Format all Rust code
fmt:
    cargo fmt

# Run clippy lints (warnings only, not blocking)
clippy:
    cargo clippy

# Run clippy with -D warnings (strict mode — may fail with preexisting debt)
clippy-strict:
    cargo clippy -- -D warnings

# Run all workspace tests
test:
    cargo test

# Check compilation (fast feedback)
check:
    cargo check

# Run fmt, clippy, test in sequence
ci:
    cargo fmt --check
    cargo clippy
    cargo test

# ── UI (Leptos + Trunk + Tailwind) ───────────────────────────────────

# Install UI toolchain (trunk + npm deps)
ui-deps:
    @which trunk >/dev/null 2>&1 || cargo install trunk
    cd crates/quilt-ui && npm install

# Build Tailwind CSS (needed before trunk serve/build)
ui-tailwind:
    cd crates/quilt-ui && npm run tailwind:build

# Run dev server (requires ui-deps first)
ui-dev:
    @which trunk >/dev/null 2>&1 || (echo "trunk not found — run 'just ui-deps' first" && exit 1)
    cd crates/quilt-ui && npm run tailwind:build && trunk serve

# ── E2E (Playwright smoke tests) ────────────────────────────────────

# Install E2E dependencies (Playwright + chromium)
e2e-deps:
    cd e2e && npm install
    npx playwright install chromium --with-deps

# Run E2E smoke tests (requires trunk dev server running on :8080)
# Start with: `just ui-dev` in another terminal
e2e:
    cd e2e && npx playwright test

# ── Housekeeping ─────────────────────────────────────────────────────

# Clean build artifacts
clean:
    cargo clean
    rm -rf crates/quilt-ui/dist

# Show available commands
default:
    @just --list
