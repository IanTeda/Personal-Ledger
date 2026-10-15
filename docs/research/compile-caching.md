# Research: compile caching with sccache and rust-cache

**Question** (#656, under the #652 map "Improve local build cycle and dev time"). Would `sccache` locally and `Swatinem/rust-cache` in CI help this workspace, given the `sqlx` query macros, `gpui` and `jdx/mise-action`? The primary metric is the warm one-file rebuild of a Desktop UI test binary (3.5 s for a test file, 9.7 s for a source file, per #652's profiling). Stable toolchain only (Rust 1.98.0, `mise.toml`). No code or config was changed and CI was not measured.

**Sources.** sccache `README.md`, `docs/Rust.md`, `docs/Local.md`, `docs/GHA.md`, `docs/Configuration.md`, `src/compiler/rust.rs` and its issue tracker (`https://github.com/mozilla/sccache`, release v0.18.0, 2026-09-14); rust-cache `README.md`, `CHANGELOG.md` and `src/config.ts`/`src/restore.ts` (`https://github.com/Swatinem/rust-cache`, v2.9.2); mise-action `README.md` (`https://github.com/jdx/mise-action`, v5.1.2); sccache-action `README.md` (`https://github.com/Mozilla-Actions/sccache-action`, v0.0.11); `sqlx-macros-core` 0.8.6 source (the version in `Cargo.lock`, read from `~/.cargo/registry`); Cargo's profiles reference (`https://doc.rust-lang.org/cargo/reference/profiles.html`) and `src/util/workspace.rs` (`https://github.com/rust-lang/cargo`); GitHub's dependency caching reference (`https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching`); this repo's GitHub Actions cache listing (`gh api repos/IanTeda/Personal-Ledger/actions/caches`, read 2026-10-15); and this repo's `mise.toml`, `Cargo.toml`, `.cargo/config.toml` and `.github/workflows/`.

## Summary

- **Local sccache: reject.** It does nothing for the primary metric. Workspace crates are built with incremental compilation in the dev profile, and sccache refuses to cache any incremental compile; the test binary is a linked crate type, which sccache also refuses; and on a warm `target/` Cargo does not invoke `rustc` for dependencies at all, so there is nothing for sccache to serve. It could only speed the cold dependency build of a fresh `target/` (new worktree, `cargo clean`), and even there Rust ignores `SCCACHE_BASEDIRS` today, proc macros and build-script runs are never cached, and crates whose cache key carries `OUT_DIR` miss across worktrees.
- **rust-cache in CI: adopt for the per-push jobs.** It is already used in `build-publish-desktop.yaml` (saving 1.5 to 1.9 GB per OS), but not in `lint-main-push.yaml` (clippy) or `end-to-end-desktop.yaml` (UI tests), which rebuild gpui and every dependency on each run. It must run after `jdx/mise-action` (it reads `rustc -vV` for its key). Save from `main` only; give each job its own key (the default); consider `cache-on-failure: true` for the UI test job.
- **sccache in CI: reject, and do not combine it with rust-cache.** Its GitHub Actions backend has open slowness and rate-limit issues, it adds nothing rust-cache does not already cover for dependencies, and the one thing it would add, caching workspace crates once rust-cache turns incremental off, is exactly where the `sqlx` macros make a stale cache hit possible.

## Facts about this workspace

- `[profile.dev]` sets only `debug = "line-tables-only"`, so `incremental` keeps Cargo's dev default of `true`; `[profile.dev.package."*"]` sets `opt-level = 1`, `debug = false` for non-members (`Cargo.toml`). Cargo applies incremental "only ... for workspace members and 'path' dependencies" (Cargo profiles reference). The `[patch.crates-io]` path dependency `vendor/stacksafe-macro` is a `proc-macro = true` crate.
- `.cargo/config.toml` sets `linker = "clang"` and `rustflags = ["-C", "link-arg=-fuse-ld=mold"]` for `x86_64-unknown-linux-gnu`.
- Build scripts exist in `bin-desktop`, `bin-tui`, `lib-colour-theme`, `lib-locale` and `lib-rpc`; `lib-database` has none.
- `sqlx-macros-core` 0.8.6 `src/query/mod.rs`: on stable, the macro reads `DATABASE_URL`, `SQLX_OFFLINE` and `SQLX_OFFLINE_DIR` through `std::env::var`, and calls `proc_macro::tracked_env::var` / `proc_macro::tracked_path::path` (which tell `rustc` about the dependency) only under `#[cfg(procmacro2_semver_exempt)]`, a nightly-only configuration. It also parses the `.env` itself (`load_dot_env`), connects to the live database when not offline, and otherwise reads `query-<hash>.json` from `SQLX_OFFLINE_DIR`, the crate's `.sqlx` or the workspace `.sqlx`. None of these inputs reach `rustc`'s dep-info on stable.
- Workflows: `lint-main-push.yaml` (clippy, `SQLX_OFFLINE=true`) and `end-to-end-desktop.yaml` (`mise run end-to-end-ui`) run on push to `main` and on pull requests to `main`; neither caches the build. `build-publish-desktop.yaml` and `build-publish-tui.yaml` run on release or `workflow_dispatch`; the Desktop one uses `Swatinem/rust-cache@v2` after `jdx/mise-action@v4` and `rui314/setup-mold@v1`. All Rust jobs install the toolchain with `jdx/mise-action@v4` (latest is v5.1.2).
- The repo's Actions cache on 2026-10-15 held 18 entries, 6.29 GB of the 10 GB limit. The rust-cache entries from `build-publish-desktop.yaml` are `v0-rust-build-test-Linux-x64-…` 1,913 MB, `…-Darwin-arm64-…` 1,537 MB and `…-Windows_NT-x64-…` 1,745 MB, all on `refs/heads/concept`. mise-action's own entries are 86 to 124 MB each.
- Locally, `target/` is 839 GB (`target/debug/deps` 696 GB, `target/debug/incremental` 139 GB), and six extra git worktrees live under `.claude/worktrees/`, each with its own `target/`.

## 1. sccache locally

### What sccache caches for Rust

- Setup is `RUSTC_WRAPPER=sccache` or `build.rustc-wrapper` in Cargo config (README). The local disk cache defaults to 10 GB (`SCCACHE_CACHE_SIZE`, `docs/Local.md`).
- **Incremental compiles are never cached.** `src/compiler/rust.rs` returns `cannot_cache!("incremental")` for any `-C incremental` argument, with the comment that "just letting rustc do its work in incremental mode is likely to be faster than trying to fetch a result from cache anyway" (upstream issue #236, still open). The README: "Incrementally compiled crates cannot be cached. By default, in the debug profile Cargo will use incremental compilation for workspace members and path dependencies." Here that means every workspace crate in a dev build goes straight to `rustc`; only registry and git dependencies are candidates.
- **Linked crate types are never cached.** Any crate type other than `rlib`/`staticlib` hits `cannot_cache!("crate-type", …)` "because rustc invokes the system linker to link them, and we don't know about all the linker inputs" (`rust.rs`); `docs/Rust.md` lists `bin`, `dylib`, `cdylib` and `proc-macro`. So the Desktop UI test binary, the `bins`, all build-script executables, and every proc-macro crate (`sqlx-macros`, `serde_derive`, gpui's macros, the vendored `stacksafe-macro`) are rebuilt by `rustc` each time. Running a build script is Cargo's job, not a `rustc` call, so it is never cached either.
- **Linking is not cached.** Follows from the above: sccache stores `rlib`/`rmeta` outputs, so the mold link of the test binary is untouched.
- **Check builds are cacheable.** `--emit=metadata` is allowed (`ALLOWED_EMIT` is `link`, `metadata`, `dep-info`), so `cargo clippy`/`cargo check` of dependencies can hit.
- **Cache key** (`rust.rs`, `generate_hash_key`): the compiler's shared libraries and version, the full argument list (minus `--extern`, `-L`, `--check-cfg`, `--out-dir`, `--diagnostic-width`; `--cfg` sorted), content hashes of every source file in `rustc --emit=dep-info`, of every `--extern` file and static lib, the env vars listed in dep-info, every `CARGO_*` env var except `CARGO_MAKEFLAGS`, `CARGO_REGISTRIES_*`, `CARGO_BUILD_JOBS` and `CARGO_ENCODED_RUSTFLAGS`, and the working directory.

### Effect on the warm one-file rebuild (the primary metric)

None. On a warm `target/`, Cargo's fingerprints keep every dependency fresh and Cargo does not call `rustc` (or the wrapper) for them. The crates Cargo does rebuild after a one-file edit are workspace crates (incremental, uncacheable) and the test binary (linked, uncacheable), followed by the mold link (not a sccache concern). sccache can only add per-invocation wrapper overhead to those calls; it cannot remove any work.

Turning incremental off to make workspace crates cacheable would trade away the incremental reuse that keeps the source-edit rebuild at 9.7 s: the edited crate always misses (its source changed) and must then compile from scratch.

### When it could help locally

Only when a `target/` is cold but the same dependency graph was built before: a new git worktree, `cargo clean`, or a deleted `target/`. Branch switches within one checkout mostly do not need it: Cargo keeps the old artifacts side by side in `target/`, keyed by `-C metadata`.

Even then, cross-checkout hits are partial:

- sccache "requires absolute paths to match for cache hits" (README, Known Caveats). `SCCACHE_BASEDIRS` (v0.14.0) strips a base directory, but `src/compiler/rust.rs` does not use it; Rust support is still open work (sccache #2595 "SCCACHE_BASEDIRS support for Worktree workflow", PRs #2794 and #2813).
- For registry dependencies, Cargo passes the absolute source path and sets the working directory to the package root in `~/.cargo/registry/src` (`path_args` in Cargo's `src/util/workspace.rs`), which is identical across worktrees, so a dependency without a build script can hit.
- A dependency whose build script output is read through `env!("OUT_DIR")` lists `OUT_DIR` in its dep-info env vars, which sccache hashes; `OUT_DIR` sits inside each worktree's `target/`, so those crates miss. gpui's dependency tree has many build scripts.
- Every crate that depends on a proc macro hashes the proc-macro `.so` it was given with `--extern`, and the proc macro is rebuilt in each `target/`. sccache PR #2882 (open, 2026-10-07) reports that on macOS the linker embeds the output path, so the same proc macro in two target directories hashes differently and "every crate that uses the macro misses the cache". Whether Linux with mold produces byte-identical proc-macro `.so` files across target directories was not verified here.

### Interaction with the `sqlx` macros

- `lib-database` is a workspace crate, so in local dev builds it is compiled incrementally and sccache never caches it. No stale-hit risk locally under the current profile.
- If incremental were turned off (or under `CARGO_INCREMENTAL=0`), the risk becomes real: the macro's inputs (`DATABASE_URL`, `SQLX_OFFLINE`, `.env`, the live database schema, the `.sqlx/*.json` files) are invisible to dep-info on stable (see Facts), so sccache would not hash them. A schema change that leaves the Rust source unchanged would return the old expansion from cache. `docs/Rust.md` warns of this class: "Procedural macros that read files from the filesystem may not be cached properly."
- Cargo itself has the same blind spot within one `target/` (`lib-database` has no build script with `rerun-if-changed`), but a fresh `target/` or `cargo clean` currently forces a correct rebuild; sccache would remove that escape hatch.

### Interaction with mold, clang and `RUSTFLAGS`

- The `-C link-arg=-fuse-ld=mold` from `.cargo/config.toml` reaches `rustc` as an argument, so it is part of every sccache key; it is constant, so it does not cause misses. Changing it (or any `RUSTFLAGS`) invalidates every entry. `CARGO_ENCODED_RUSTFLAGS` is skipped as an env var because it is "already cached in argument list" (`rust.rs`).
- The linker never runs for cacheable crate types (`rlib`/`rmeta`), so mold's speed gain and sccache do not overlap.
- `mise.toml` exports `CARGO_TERM_COLOR=always`, which sccache hashes as a `CARGO_*` var; it must stay the same across shells and tools (bacon, rust-analyzer) for their builds to share entries.

## 2. rust-cache in CI

### What it caches and prunes

- Caches `~/.cargo` (registry index and `.crate` archives, git dependencies, `bin/`, `.crates.toml`, `.crates2.json`) and `./target`; not `~/.cargo/registry/src`, which Cargo re-extracts faster (README, `src/config.ts`).
- Before saving it removes pre-existing `~/.cargo/bin` files, dependencies no longer used, "anything that is not a dependency", incremental artifacts, and artifacts with an mtime older than a week. "The workspace crates themselves are not cached since doing so is generally not effective", unless `cache-workspace-crates: true` (since v2.8.0) (README, CHANGELOG).
- It exports `CARGO_INCREMENTAL=0` for later steps (`src/restore.ts`).

### Cache key

From `src/config.ts`: `<prefix-key, default v0-rust>-<shared-key | key + GITHUB_JOB>-<os.type()>-<arch>-<env hash>-<lockfile hash>`.

- The job id (`GITHUB_JOB`) is added unless `shared-key` is set or `add-job-id-key: false`.
- The env hash covers the output of `rustc -vV` (and of each toolchain from `rustup toolchain list`, since v2.9.0) plus every env var whose name starts with `CARGO`, `CC`, `CFLAGS`, `CXX`, `CMAKE` or `RUST` (extendable with `env-vars`). `SQLX_OFFLINE` is not included; that is fine because `lib-database` is a workspace crate and is rebuilt every run.
- The lockfile hash covers workspace members' `Cargo.toml`/`Cargo.lock`, `rust-toolchain(.toml)` and `.cargo/config.toml` (README). Editing the mold settings therefore starts a new cache.
- On a miss it restores the newest entry matching the key without the lockfile hash, so a `Cargo.lock` change rebuilds only changed dependencies (README). An exact hit is not re-saved (caches are immutable).

### Interaction with mise-action

- The README requires the toolchain to be selected before the action, "as the cache uses the current rustc version as its cache key". It runs `rustc -vV` from `PATH` (`cmd-format` default `{0}`), so `jdx/mise-action` (which adds mise's PATH entries for later steps by default, `export_path: true`) must come first. The existing Desktop workflow already has this order. Keep `rui314/setup-mold` before the build step; its position relative to rust-cache does not affect the key.
- mise-action exports the `[env]` from `mise.toml` (`env: true` default): `RUST_BACKTRACE=1` and `CARGO_TERM_COLOR=always` both match rust-cache's prefixes and land in the key. They are constant, so harmless, but changing them in `mise.toml` starts new caches.
- mise-action's own cache (`cache: true` by default) holds only mise's data directory (`~/.local/share/mise`). Because mise installs Rust through rustup, "rustup keeps the toolchain, components … in `~/.rustup` and `~/.cargo`" outside that directory; with mise 2026.9.2+ `mise install` notices and re-downloads it every run. mise-action's README names rust-cache as the complement: it "caches the cargo registry and `target/`, not the toolchain, so it works alongside mise managing Rust". Caching the toolchain too (`MISE_RUSTUP_HOME`/`MISE_CARGO_HOME` inside the mise data dir) costs "around 2.5 GB" (mise-action README, "Rust Cache").
- Overlap: rust-cache's `cache-bin` caches `~/.cargo/bin`, but cleans out files present before it ran (the rustup proxies). mise's `cargo:` tools (`sqlx-cli`, `bacon`) live under the mise data dir, so they are not cached twice.
- mise-action's README warns: "Don't restore a cache in a workflow that publishes artifacts", because a lower-trust workflow (`pull_request_target`, `workflow_run`, `issue_comment` running untrusted code) that can write the default branch's cache could poison a release; an ordinary `pull_request` writes only to its merge-ref scope. `build-publish-desktop.yaml` restores both the mise cache and rust-cache in its release job. This repo has no such lower-trust workflows today, so it is a rule for the decision ticket to keep, not a current hole.

### Scope, size and saving

- GitHub: a run can restore caches from its own branch or the default branch; a pull request also from its base branch, and its own caches are written to `refs/pull/<n>/merge`; sibling branches cannot see each other's caches. 10 GB per repository by default (user-owned repos can raise it, at extra cost), least recently used evicted first, entries unused for 7 days removed (GitHub dependency caching reference).
- Consequence for `concept`: the existing `concept` rust-cache entries (5.2 GB in total) are invisible to runs on `main` and to pull requests into `main`, and `concept` pushes do not trigger the lint or UI-test workflows. Caches saved from `main` are readable by every branch, `concept` included.
- Size: the Linux Desktop entry, which holds dependency artifacts for both a debug and a release build of `bin_desktop`, is 1.9 GB compressed. A UI-test job (debug, plus `test-support` and dev-dependencies) and a clippy job (metadata only) should each be in that range or smaller (not measured). Adding both on `main` next to today's 6.3 GB would approach 10 GB, and LRU eviction would then drop the oldest, likely the `workflow_dispatch` packaging caches.
- `save-if` (default `true`) and `cache-on-failure` (default `false`; failing jobs do not save) are inputs (README, `src/restore.ts`).

## 3. sccache in CI versus rust-cache

- `mozilla-actions/sccache-action` installs sccache and needs `SCCACHE_GHA_ENABLED=true` and `RUSTC_WRAPPER=sccache` (sccache-action README). The GHA backend stores entries in GitHub's cache under the same 10 GB limit; `SCCACHE_GHA_VERSION` purges it; on hitting the service's rate limit "the build will continue, but the storage might not be performed" (`docs/GHA.md`).
- Open upstream issues on that backend: #1485 "GitHub Actions cache is surprisingly slow", #1756 "GHA backend is pretty slow", #2593 (a rate-limited cache check switches the cache to read-only), #2821 (writes dropped on HTTP 429).
- Coverage: sccache still rebuilds proc macros, build-script executables, build-script runs and the final link, all of which rust-cache restores as part of `target/`. rust-cache does not cache workspace crates; sccache could (CI has no incremental, especially once rust-cache sets `CARGO_INCREMENTAL=0`), and that is the one gain. For this workspace it brings the `sqlx` stale-hit risk from section 1 into CI: `lint` and the UI tests compile `lib-database` against `.sqlx` with `SQLX_OFFLINE=true`, and a regenerated `.sqlx` with unchanged Rust source would not change sccache's key, so CI could pass on a stale expansion.
- Combining: neither project documents running both together. With rust-cache restoring `target/`, Cargo finds dependencies fresh and never calls sccache for them, so the combination only adds the workspace-crate caching above.

## 4. Recommendation and facts for the "CI job layout" decision

**Local sccache: reject.** Zero effect on the warm one-file rebuild, which is all workspace-crate incremental compilation and linking. Its only win, cold dependency builds in new worktrees, is limited until sccache's Rust path honours `SCCACHE_BASEDIRS` (#2595, #2794, #2813); revisit then if worktree cold builds become the bottleneck.

**rust-cache in CI: adopt** for `lint-main-push.yaml`'s `lint` job and `end-to-end-desktop.yaml`'s `end-to-end-ui` job. Not for `deny` (cargo-deny does not compile).

- Step order: `actions/checkout` → `jdx/mise-action` → `rui314/setup-mold` → `Swatinem/rust-cache@v2` → build/test. rust-cache needs `rustc` on `PATH` when it runs.
- Keys: keep the default job-id key, so `lint` (clippy, metadata) and `end-to-end-ui` (full test build with `test-support`) get separate caches; their artifacts differ, so a `shared-key` between them would only thrash. `end-to-end-desktop.yaml` is also called by `build-publish-desktop.yaml` (`workflow_call`), where `GITHUB_JOB` is still `end-to-end-ui`, so both callers share one key family.
- Saving: `save-if: ${{ github.ref == 'refs/heads/main' }}`, so pushes to `main` write and pull requests read `main`'s cache without writing merge-ref copies that nothing else can read. If `concept` pushes ever trigger these workflows, they would read `main`'s cache but should not save, for the same reason.
- `cache-on-failure: true` on the UI-test job, so a failing test on `main` still saves the dependency build (dependencies compiled fine even when a test fails).
- Budget: about 6.3 GB is already used, mostly by `workflow_dispatch` packaging caches on `concept`. Decide whether those workflows should keep saving (for example `save-if` on `release` events only), or accept LRU eviction.
- Keep the mise-action rule: do not restore caches in publishing jobs if a lower-trust workflow is ever added.
- Optional: bump `jdx/mise-action@v4` to v5 (v5.1.2 current); not required for rust-cache.

**sccache in CI: reject**, alone or combined with rust-cache.

## Gaps

- No timings were taken; the CI gain is reasoned from sources and from the existing Desktop workflow's cache sizes.
- Whether Linux proc-macro `.so` files are byte-identical across target directories (which decides cross-worktree sccache hits for their dependants) was not tested.
- The sizes of rust-cache entries for the clippy and UI-test jobs are estimates.
