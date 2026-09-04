# Research: Is Chrono deprecated, and what would replace it?

**Question.** `docs/product-requirements.md` CC-ALL-003 and §5 Dependencies both assert Chrono is deprecated and call for researching a replacement. This note checks that premise against primary sources first, then — regardless of the answer — surveys the two credible alternatives (`jiff`, `time`) against this workspace's actual usage: `chrono::DateTime<Utc>` timestamps in `lib-domain` (`RowID`, `HybridLogicalClock`), every `lib-database` entity model (`categories`, `accounts`, `change_sets`, all via `sqlx`), `lib-rpc`'s proto timestamp mapping, and `bin-sync-server`'s JWT `exp`/`iat` claims (`src/auth/jwt.rs`).

Research was done by querying the crates.io API directly for `chrono`, `jiff`, `time`, and `sqlx-sqlite` version/feature metadata, reading `chrono`'s own README and GitHub repository, checking the RustSec Advisory Database's `crates/chrono` directory for every advisory ever filed against the crate, and reading `jiff`'s own README and `COMPARE.md` (its maintainer-authored comparison against `chrono`/`time`).

## 1. Is Chrono actually deprecated?

No evidence of deprecation was found anywhere. Specifically:

- **crates.io metadata**: `chrono`'s `newest_version`/`max_stable_version` is `0.4.45`, last published `2026-06-04` (three months before this research), with 779,448,819 cumulative downloads and the plain description "Date and time library for Rust" — no deprecation banner, no `readme_deprecation` field, nothing (`https://crates.io/api/v1/crates/chrono`, queried directly).
- **The repository itself** (`https://github.com/chronotope/chrono`): actively maintained — 157 open issues, 37 open PRs, 1,784 commits, recent CI activity. No statement anywhere on the repo that development has stopped or moved elsewhere.
- **The README** (`https://raw.githubusercontent.com/chronotope/chrono/main/README.md`): describes an actively developed library, discusses current features (e.g. opt-in `rkyv` serialization support), and documents an explicitly-tested MSRV in CI. It recommends the *companion* crates `chrono-tz`/`tzfile` for full IANA timezone-database support — a deliberate scope choice to keep the core crate's binary size down, not a deprecation signal.
- **RustSec Advisory Database** (`https://github.com/rustsec/advisory-db/tree/main/crates/chrono`): exactly one advisory has ever been filed against `chrono` itself — **RUSTSEC-2020-0159**, "Potential segfault in `localtime_r` invocations" (2020-11-10, tied to CVE-2020-26235/RUSTSEC-2020-0071, the same underlying `time`-crate-era Unix `localtime_r` soundness issue). It was **fixed in chrono 0.4.20**. This workspace already pins `chrono = { version = "0.4.42", features = ["serde", "clock"] }` in the root `Cargo.toml`, and `Cargo.lock` currently resolves it to `0.4.45` — both far past the fixed version. No other advisory, open or closed, exists against `chrono`.

**Conclusion: the PRD's premise doesn't hold up.** Chrono is not deprecated — it's an actively released, widely used crate (the second-most-downloaded of the three date/time crates checked here) with a single, years-old, long-fixed advisory that this workspace's pinned version is already unaffected by.

## 2. The alternatives anyway: `jiff` and `time`

Surveyed as due diligence regardless of §1's finding:

**`jiff`** (crates.io: `jiff`, created by BurntSushi — the `regex`/`ripgrep` author): first released 2024-02-17, currently `0.2.35`, last published `2026-07-25`, 178,565,834 downloads. Its README describes it as "heavily inspired by the Temporal project" (the TC39 proposal for improved JS datetime handling), aiming to be "difficult to misuse." Its own `COMPARE.md` (`https://github.com/BurntSushi/jiff/blob/master/COMPARE.md`) — a maintainer-authored, direct comparison against `chrono` — makes **no deprecation or migration claim**; it presents `jiff` as technically differentiated, not as a mandated successor:
  - *Timezone-aware serde round-tripping*: "Chrono only serializes the offset, which makes lossless deserialization impossible. Chrono loses the time zone information," where `jiff` round-trips losslessly.
  - *Calendar arithmetic*: `jiff` can "produce spans of time involving days between two zone aware datetimes that is consistent with adding days" in cases `chrono` doesn't fully support.
  - *Duration rounding*: `jiff` supports DST-safe rounding of durations themselves (not just datetimes), which `chrono` doesn't.
  - Supports serde ("opt-in Serde support" per its README).

**`time`** (crates.io: `time`): mature, currently `0.3.55`, last published `2026-08-01`, 871,303,901 downloads — the most-downloaded of the three. Supports serde.

Neither survey found any Windows/macOS/Linux-specific cross-compilation problem for any of the three crates (NFR.6a) — all are pure-Rust with no OS-specific FFI in their core datetime types, and this workspace already cross-compiles `chrono` successfully today across the TUI's and Desktop's packaging targets.

## 3. `sqlx` integration — the deciding factor for this workspace

`sqlx-sqlite`'s own published Cargo features were checked directly against both the version this workspace pins (`sqlx = "0.8.6"`) and the current latest (`0.9.0`, as of this research) via the crates.io API:

```
"chrono": ["dep:chrono", "sqlx-core/chrono"],
"time":   ["dep:time",   "sqlx-core/time"],
```

Both `chrono` and `time` have **first-party `sqlx-sqlite` features** — enabling one wires up `sqlx::Type`/`Encode`/`Decode` for that crate's timestamp types against SQLite automatically, which is exactly how this workspace's `lib-database` uses `chrono::DateTime<Utc>` columns today (`categories`, `accounts`, `change_sets` models).

**`jiff` has no such feature in either `sqlx-sqlite` version** — none was found, in `0.8.6` or `0.9.0`. Adopting `jiff` would mean this workspace hand-rolling `sqlx::Type`/`Encode`/`Decode` impls for every `jiff` timestamp type used against SQLite (or converting to/from `chrono`/`time` at every database read/write boundary), with no upstream `sqlx` support to lean on — an ongoing integration cost, not a one-time migration cost, since it would need maintaining across future `sqlx` upgrades too.

## 4. Recommendation

**Keep Chrono.** Its deprecation was never real (§1) — RUSTSEC-2020-0159 is fixed and years behind the version already pinned here — so CC-ALL-003's stated reason for replacing it doesn't hold. `jiff`'s technical advantages (§2) are real but address problems this codebase doesn't currently have (this workspace stores UTC timestamps, not zone-aware wall-clock times with DST-fold ambiguity), and adopting it would trade a working, first-party `sqlx` integration for a hand-rolled one (§3) across every timestamp-bearing table in `lib-database`, plus `lib-rpc`'s proto mapping and `bin-sync-server`'s JWT claims — real, ongoing cost for no problem currently in scope. `time` is a viable alternative on paper (first-party `sqlx` support, comparable maturity) but switching to it would still mean touching all 16+ files that reference `chrono::` today for no functional gain identified in this research. No replacement is justified by anything found here; CC-ALL-003's underlying "chrono is deprecated" assumption is what should be corrected, not the crate.
