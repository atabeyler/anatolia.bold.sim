# Builds and runs sim-server + the static client build as a single Fly.io
# machine, mirroring exactly what Render's build container already does:
# the root `npm run build` script (build:wasm -> client -> cargo build -p
# sim-server --release). See AGENTS.md's WASM-Local Mode / Dev Commands
# sections for why that specific order matters.
FROM rust:1-bookworm AS builder

# Node.js 20 (package.json requires >=18) -- the official rust image has no
# Node.js of its own.
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# scripts/build-wasm.mjs installs its own pinned nightly toolchain
# (rust/sim-wasm/rust-toolchain.toml) via rustup itself -- nothing extra
# needed here beyond a writable CARGO_HOME, which this build stage already
# has (unlike Render's read-only one, see that script's own comments).
RUN npm run build

# ---- runtime: a minimal image with just the compiled binary + static assets ----
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/rust/target/release/sim-server ./sim-server
COPY --from=builder /app/client/dist ./client/dist

ENV NODE_ENV=production
# sim-server defaults to 3002 when PORT is unset (rust/sim-server/src/main.rs) --
# set explicitly here so it's never ambiguous which port fly.toml must match.
ENV PORT=3002
EXPOSE 3002

CMD ["./sim-server"]
