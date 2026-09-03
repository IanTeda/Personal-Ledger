# Deployment

This page covers deploying the **Sync Server** — the only component of Personal Ledger that's actually deployed anywhere; the Desktop and TUI apps are installed locally as native binaries (see their own packaging notes in `docs/directories-files.md`). It assumes some familiarity with the architecture below; if you're only interested in `docker compose up`, skip to [Quick start](#quick-start).

## Architecture

Personal Ledger is not a client/server app in the traditional sense — there's no central database every Client reads and writes against. Instead:

- **Ledger**: the complete set of Accounts, Categories, and Transactions owned by one self-hoster. It's a logical whole, physically replicated in full on every Client.
- **Client** (Desktop or TUI): an app instance holding its own full local copy of the Ledger in an embedded SQLite database, capable of every read/write entirely offline. Neither app depends on the Sync Server to function day to day.
- **Sync Server**: a separate, headless component — not a Client, and it never holds a full Ledger copy — that lets Clients propagate their local edits to each other. It does this by exchanging Change Sets (field-level edits) with each Client, not by exposing Ledger CRUD directly, and it keeps its own durable log of every Change Set so a Client that's been offline can catch up without any other Client being online at the same time.

```text
┌─────────────┐                              ┌─────────────┐
│   Desktop    │                              │     TUI      │
│  (own full   │◄──┐                      ┌──►│  (own full   │
│  SQLite copy)│   │                      │   │  SQLite copy)│
└─────────────┘   │                      │   └─────────────┘
                   │   push/pull Change   │
                   │   Sets over gRPC     │
                   │   (bearer JWT)       │
                   ▼                      ▼
              ┌─────────────────────────────────┐
              │            Sync Server            │
              │   (headless, Docker-deployed)     │
              │  durable Change Set log (SQLite)  │
              └─────────────────────────────────┘
```

This is why the Sync Server is deployed at all: it's the one component meant to run continuously, unattended, on infrastructure you control — a homelab, a NAS, a small always-on server — rather than on the same device you're actively using the Desktop or TUI app on. See `CONTEXT.md`'s glossary for the precise definitions and `docs/adr/0009-lww-sqlite-change-set-log.md`/`docs/adr/0010-oauth2-pkce-native-app-auth.md` for how conflict resolution and authentication actually work.

### Trusted network assumption

The Sync Server is assumed to run within your own trusted network (a homelab, not exposed directly to the public internet). Its authentication (OAuth2 Authorization Code + PKCE, see ADR-0010) protects against other devices on that trusted network, not against a hostile network — there's no TLS on the gRPC/HTTP listener itself in this cycle. Don't port-forward it to the public internet without adding your own transport-layer protection (a reverse proxy with TLS, a VPN/tailnet, etc.).

### Current limitations

This is feasibility-cycle work (see the [Sync Server feasibility map](https://github.com/IanTeda/Personal-Ledger/issues/40)), not a finished product. Concretely:

- **Desktop and TUI don't actually sync yet.** Both apps demonstrate their own embedded-SQLite storage today, but neither has the Client-side wiring (the browser-login flow, OS-keychain token storage, the actual push/pull calls) to talk to a Sync Server — that's real application integration deliberately left out of this feasibility cycle (see the auth ticket's own scope notes). The architecture and the Sync Server side of the protocol are proven end to end (see the map's integration tests); the Client apps themselves aren't wired up to use it yet.
- **One hardcoded account.** The Sync Server bootstraps a single account (`admin` / `change-me`) on first run if none exists — see `crates/bins/bin-sync-server/src/main.rs`'s `BOOTSTRAP_USERNAME`/`BOOTSTRAP_PASSWORD`. There's no config-driven way to set a real password yet, and multi-account support doesn't exist (ADR-0010 fixes this cycle to a single self-hoster's own account). Don't rely on this for anything beyond local experimentation until credential provisioning is addressed.
- **Backup/restore isn't specified yet** (PRD NFR.8 placeholder) — the persistent volume (below) survives container restarts, but there's no documented backup strategy for it beyond whatever you already do for the Docker host itself.

## The Docker image

`Dockerfile.sync-server` (repo root) builds a multi-arch (`linux/amd64`, `linux/arm64`) image: a `rust:1.98.0-bookworm` build stage, and a `debian:bookworm-slim` runtime stage that never runs the Sync Server process as root. See `.github/workflows/build-publish-sync-server.yaml` for how it's built, verified (a real container run plus a `grpcurl` call, on both architectures, on every change), and — on a GitHub Release — published to `ghcr.io/ianteda/personal-ledger-sync-server`.

### Non-root and `PUID`/`PGID`

The container always runs the Sync Server process as a non-root user. By default that's a fixed uid/gid (`10001`), but it's runtime-configurable via the `PUID`/`PGID` environment variables (the same convention linuxserver.io images use) — set them to match whatever user already owns your host-side volume if you want to inspect the durable store's files directly from the host:

```sh
docker run -e PUID=1000 -e PGID=1000 ...
```

`docker-entrypoint.sync-server.sh` does the actual work: it starts as root just long enough to reshape the baked-in account to the requested `PUID`/`PGID` and fix up ownership of the durable store, then drops privileges via `setpriv` before ever executing the Sync Server binary itself.

### Ports and storage

- **`50051`** — the gRPC (and, for the `/authorize`/`/token` OAuth2 endpoints, HTTP) listener. One port serves both; see ADR-0010.
- **`/data`** — where the durable Change Set log (and the account store) lives, as a single SQLite file. Mount a volume here if you want it to survive anything more than a container restart on its own filesystem layer.

## Quick start

The example deployment lives at [`example/compose.sync-server.yaml`](https://github.com/IanTeda/Personal-Ledger/blob/feasibility/example/compose.sync-server.yaml) — it's an example, not the only way to run this, but it's the recommended starting point. It's standalone: it pulls the pre-built multi-arch image from GHCR, so you don't need to clone the repo or run it from any particular directory — just download the one file and start it:

```sh
curl -O https://raw.githubusercontent.com/IanTeda/Personal-Ledger/feasibility/example/compose.sync-server.yaml
docker compose -f compose.sync-server.yaml up -d
```

This pulls `ghcr.io/ianteda/personal-ledger-sync-server:latest`, starts the Sync Server on port `50051`, and creates a named volume (`sync-server-data`) for its durable store — seeded from the image's own pre-created (empty) database file on first run, so there's no separate init step.

Confirm it's actually serving requests (matches how this repo's own CI verifies it):

```sh
grpcurl -plaintext \
  -import-path crates/libs/lib-rpc/proto \
  -proto personal-ledger/v001/utilities.proto \
  localhost:50051 \
  personal_ledger.utilities.v001.UtilitiesService/Ping
```

To override the non-root `PUID`/`PGID` (see above), either export them before running Compose or drop them in a `.env` file next to the compose file:

```sh
PUID=1000 PGID=1000 docker compose -f compose.sync-server.yaml up -d
```

### Stopping and restarting

`docker compose -f compose.sync-server.yaml restart` (or `down` followed by `up -d` again) keeps the named volume, so the durable store — including the bootstrapped account — survives. Only `down -v` removes it; that's a deliberate, explicit action, not something that happens by accident.

## Building the image yourself

If you'd rather not wait for a published `ghcr.io` tag, build directly from the Dockerfile:

```sh
docker buildx build --file Dockerfile.sync-server --tag personal-ledger-sync-server:local .
```

Build context is the repo root (see the Dockerfile's own header comment) — Cargo needs every workspace member's `Cargo.toml` present to resolve the workspace, even though only `bin-sync-server` actually gets compiled.
