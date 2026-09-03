# Dockerfile for the Personal Ledger Sync Server (FC-SYNC-005, NFR.6b).
#
# Multi-arch (linux/amd64, linux/arm64) via `docker buildx build --platform ...` --
# see .github/workflows/build-verify-sync-server-docker.yaml, which builds each
# architecture natively on its own GitHub-hosted runner rather than cross-compiling or
# emulating under QEMU.
#
# Build context is the repo root (not crates/bins/bin-sync-server), because
# bin-sync-server depends on sibling crates via relative `path = "../../libs/..."`
# entries, and Cargo needs every workspace member's Cargo.toml present to resolve the
# workspace at all, even though only bin_sync_server actually gets built here.

# ---- Builder ----------------------------------------------------------------------
# Pinned to the same Rust version mise.toml pins for local dev (see [tools].rust
# there) so this build uses exactly what `mise install` would. The official image
# already ships a full C toolchain (gcc et al.), which sqlx-sqlite's bundled-SQLite
# build needs -- no extra apt package required for that part.
FROM rust:1.98.0-bookworm AS builder

# lib_rpc's build.rs (tonic_prost_build) needs a system `protoc` at build time --
# CLAUDE.md: "Building lib_rpc requires a system protoc (protobuf compiler)".
# libprotobuf-dev provides the well-known-types .proto files under /usr/include,
# which build.rs passes explicitly via --proto_path=/usr/include.
RUN apt-get update \
    && apt-get install -y --no-install-recommends protobuf-compiler libprotobuf-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace
COPY . .

# lib-database's sqlx::query!/query_as! macros need either a live DATABASE_URL or
# SQLX_OFFLINE=true against the checked-in .sqlx cache at compile time (CLAUDE.md) --
# this build environment has neither network access to a database nor one to spin up,
# so it must run offline against the cache already regenerated in #46/#50's work.
ENV SQLX_OFFLINE=true

RUN cargo build --release --locked --package bin_sync_server --bin sync-server

# ---- Runtime ------------------------------------------------------------------------
# debian:bookworm-slim matches the builder's own base distribution (avoids any glibc
# version mismatch between the two stages) while staying minimal -- sqlx-sqlite bundles
# its own SQLite at compile time (confirmed via Cargo.lock: sqlx-sqlite's default
# features enable libsqlite3-sys's "bundled" feature), so no libsqlite3 runtime package
# is needed here.
FROM debian:bookworm-slim AS runtime

# Baseline non-root account (NFR.2: "the app never requires root/administrator
# privileges to run"), a fixed high UID/GID as the default. docker-entrypoint.sh
# reshapes this account at container start to match the operator-supplied PUID/PGID
# env vars (falling back to these same defaults when unset), so the *running* process
# is always non-root even though the image itself starts as root to do that
# adjustment -- see the entrypoint script's own comment for why.
RUN groupadd --system --gid 10001 syncserver \
    && useradd --system --uid 10001 --gid syncserver --no-create-home --shell /usr/sbin/nologin syncserver

# The Sync Server's own durable Change Set log lives here. lib-database's default
# DatabaseConfig::url is the literal relative path "sqlite:./personal-ledger.sqlite"
# (see crates/libs/lib-database/src/config.rs's DEFAULT_URL), resolved against this
# WORKDIR -- pre-creating the (empty) file means the container starts with zero
# required config/env, since SQLite's driver needs the file to already exist unless
# the URL carries "?mode=rwc" (which the built-in default does not). This also sets up
# the mount point #49's persistent volume will use later; this ticket doesn't wire that
# up itself. Ownership is (re)applied by docker-entrypoint.sh on every start, since a
# bind-mounted host directory here would otherwise keep whatever ownership it already
# has, not what's set at build time.
WORKDIR /data
RUN touch /data/personal-ledger.sqlite \
    && chown -R syncserver:syncserver /data

COPY --from=builder /workspace/target/release/sync-server /usr/local/bin/sync-server
COPY docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# PUID/PGID: the runtime-configurable UID/GID docker-entrypoint.sh applies to the
# syncserver account before dropping privileges -- the same convention linuxserver.io
# images use, so a self-hoster can match the container's user to whatever UID already
# owns their host-side volume mount (relevant once #49 adds one) without an image
# rebuild. Declared here (rather than left implicit) so `docker inspect`/`--env-file`
# tooling can discover them.
ENV PUID=10001
ENV PGID=10001
EXPOSE 50051

# Stays root at the container level so the entrypoint can adjust syncserver's
# UID/GID and fix up /data's ownership before dropping privileges -- see
# docker-entrypoint.sh. The actual Sync Server process it execs into is never root.
ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
CMD ["/usr/local/bin/sync-server"]
