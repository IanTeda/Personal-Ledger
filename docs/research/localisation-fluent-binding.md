# Research: Choosing a Fluent binding for Rust

**Question.** Fluent is fixed as the message format. Which Rust crate(s) should `lib-localisation` build on, given one shared library serving `bin-desktop` (gpui 0.2.2 + gpui-component 0.5.1) and `bin-tui` (ratatui 0.30.2), with Locales `en-AU` (source), `en-US`, `en-GB` and `en-XA` (pseudo), and no UI-framework dependency in the library? Ticket: #212, child of map #211.

Method: crates.io API metadata (version, last publish, downloads), the crates' own `Cargo.toml` and source on GitHub, docs.rs pages, and the module documentation inside `fluent-langneg`. Binary size and build time were not measured; where a claim rests on inference rather than a source, it says so.

## 1. Candidates and maintenance

Downloads are cumulative, from the crates.io API queried on 2026-09-20.

| Crate | Latest | Last published | Downloads | Role |
| --- | --- | --- | --- | --- |
| `fluent-bundle` | 0.16.0 | 2025-05-22 | 18.3M | Low-level bundle for one Locale (projectfluent/fluent-rs) |
| `fluent` | 0.17.0 | 2025-05-23 | 13.3M | Facade re-exporting `fluent-bundle` |
| `fluent-langneg` | 0.14.2 | 2025-10-24 | 18.4M | Locale negotiation |
| `unic-langid` | 0.9.6 | 2025-05-09 | 23.2M | Locale identifiers |
| `fluent-pseudo` | 0.3.3 | 2025-05-22 | 0.5M | Pseudolocalisation transform |
| `i18n-embed` | 0.16.0 | 2025-07-09 | 6.4M | Loader over `fluent` (cargo-i18n) |
| `i18n-embed-fl` | 0.10.1 | 2026-07-23 | 6.2M | `fl!` macro for `i18n-embed` |
| `fluent-templates` | 0.15.1 | 2026-08-05 | 1.1M | Loader over `fluent-bundle` |
| `rust-i18n` | 4.2.2 | 2026-09-08 | 3.5M | Not Fluent (see below) |
| `fluent-resmgr` / `fluent-fallback` | 0.0.8 / 0.7.2 | 2025-05 | 0.03M / 0.5M | Mozilla-style resource manager and fallback |

The core Fluent crates are stable but quiet: the whole fluent-rs workspace was released together in May 2025 and nothing has shipped since, other than `fluent-langneg` in October 2025. That is consistent with a finished low-level layer rather than abandonment, but it is a dependency on a slow-moving upstream. `i18n-embed` declares `maintenance = { status = "actively-developed" }` in its `Cargo.toml`, and its macro crate shipped in July 2026. `fluent-templates` shipped in August 2026.

`rust-i18n` is ruled out: its README supports YAML (default), JSON or TOML for translations and makes no mention of Fluent or `.ftl`. It has fallback chains and a missing-translation extractor, but the message format is the wrong one for a project where Fluent is fixed.

`fluent-resmgr` and `fluent-fallback` (Mozilla's async, resource-manager-oriented stack) target Firefox's needs and are pre-1.0 with little download volume. They are not considered further.

## 2. Comparison of the three viable options

### 2.1 `fluent-bundle` used directly

- **Embedding**: none provided. `FluentResource::try_new` takes a `String`; the app supplies it, typically via `include_str!` (compile time) or file reads (runtime). Nothing to configure either way.
- **Compile-time id/argument checking**: none. A build script or test would be needed (see section 3).
- **Negotiation and fallback**: `fluent-langneg` is already a dependency of `fluent-bundle`. Chains must be assembled by hand: hold one `FluentBundle` per Locale and try them in order.
- **Plurals and selects**: full Fluent select expressions; plural categories come from `intl_pluralrules` (a `fluent-bundle` dependency).
- **Thread safety**: `FluentBundle` is generic over a memoizer `M`. The default uses a `RefCell`-based memoizer; `FluentBundle::new_concurrent` swaps in a `Mutex`-based one, and per the docs.rs page both are `Send` and `Sync` when their type parameters are.
- **UI framework**: none.
- **Cost**: smallest dependency set (`fluent-langneg`, `fluent-syntax`, `intl_pluralrules`, `unic-langid`, `intl-memoizer`, `rustc-hash`, `self_cell`, `smallvec`, per its `Cargo.toml`). No proc macros.

### 2.2 `i18n-embed` (feature `fluent-system`) plus `i18n-embed-fl`

- **Embedding**: assets are embedded via `rust-embed` (default feature). The README states the crate "conveniently embed[s] localization assets into your application binary", and notes the dependency on `rust-embed` "may change". Runtime filesystem loading exists behind the `filesystem-assets` feature, with `autoreload` (via `notify`) as an option.
- **Compile-time checking**: the `fl!` macro checks the message id, the optional attribute and the argument names at compile time, but only against the `fallback_language` named in the crate's `i18n.toml`, and ids must be string literals. The HashMap argument form is not checked. Per the macro's docs.rs page.
- **Negotiation and fallback**: `FluentLanguageLoader` holds one fallback language (required config). `load_languages` always appends the fallback language, and `select_languages_negotiate(requested, strategy)` calls `fluent_langneg::negotiate_languages` with the fallback as the default. Lookups walk the selected bundles in order, so a message missing from `en-GB` resolves from the next language in the list (this is visible in `select_languages`, which chains the fallback onto the chosen languages).
- **Plurals and selects**: full Fluent, since it wraps `fluent`.
- **Thread safety**: the `FluentLanguageLoader` docs.rs page lists `Send` and `Sync`; bundles use `intl_memoizer::concurrent::IntlLangMemoizer` and the source constructs them with `FluentBundle::new_concurrent`. The loader keeps its state behind `arc-swap`, so the active languages can be switched at runtime without `&mut`.
- **UI framework**: none. Optional `desktop-requester` (via `sys-locale`) and `web-sys-requester` only detect the OS Locale.
- **Cost**: adds `rust-embed`, `arc-swap`, `parking_lot`, `log`, `thiserror` and two proc-macro crates (`i18n-embed-impl`, the `fl!` macro) on top of the fluent-rs set. Needs an `i18n.toml` and an `i18n/<locale>/<domain>.ftl` layout.

### 2.3 `fluent-templates`

- **Embedding**: `static_loader!` embeds at compile time; `ArcLoader` loads at runtime. Both take a directory of Locale folders.
- **Compile-time checking**: none for ids or arguments.
- **Negotiation and fallback**: `build_fallbacks` (in its source) computes, for each loaded Locale, the list from `negotiate_languages(&[locale], locales, None, Filtering)`, then lookups also try the single `fallback_language`. The structure is per-Locale, so `en-GB` can resolve to `en-AU` (see section 4), but the macro only accepts one global `fallback_language`.
- **Plurals and selects**: full Fluent.
- **Thread safety**: `create_bundle` uses `FluentBundle::new_concurrent`; the loader is a plain shared value.
- **UI framework**: none, but the crate is oriented to template engines (`handlebars`, `tera` features, off by default). Its `Cargo.toml` shows `fluent-langneg = "0.13"` while `fluent-bundle` 0.16 uses a newer one, so two `fluent-langneg` versions may land in the tree (inferred from the manifests; not built).
- **Cost**: `ignore`, `flume`, `log` under default features (file walking at macro time), plus the macro crate. Its `create_bundle` uses `expect` on resource errors, which conflicts with this repo's no-`expect` convention only inside the dependency, not our code.

## 3. Compile-time checking without a macro

The repo would want ids and arguments checked. Options, from `fl!` outward:

- `i18n-embed-fl`'s `fl!`: ready-made, checks against the source Locale (`en-AU`) as long as `fallback_language = "en-AU"`. Limits: literal ids only, HashMap args unchecked.
- With `fluent-bundle` directly: parse the `en-AU` FTL in `build.rs` with `fluent-syntax` (already in the tree) and generate a `MessageId` enum or constants plus typed argument structs. This gives tighter checking than `fl!` (arguments become typed) at the cost of owning a generator.
- In any option, add a test that every id in `en-US`, `en-GB` and `en-XA` exists in `en-AU`, and every `$variable` matches the source message. This catches translator drift that `fl!` cannot, since it only inspects the fallback file.

## 4. `en-GB` to `en-AU` to source

`fluent-langneg`'s own module documentation lists the matching steps, and step 6 is "Attempt to look up for a different region of the same locale", with the example `["en-GB"] * ["en-AU"] = ["en-AU"]` (replace the region with a range, so `en-*` matches `en-AU`). Strategies (source enum `NegotiationStrategy`): `Filtering` returns every match in order, `Matching` returns the best match per requested Locale, `Lookup` returns a single match.

So with `Filtering`, requesting `en-GB` against available `[en-AU, en-GB, en-US, en-XA]` yields `en-GB` first (exact), then `en-AU` (step 6). Two consequences:

- The chain `en-GB -> en-AU` falls out of the negotiation; it does not need to be configured, provided `en-GB` overrides only the messages that differ and `en-AU` is complete.
- `en-US` also falls back to `en-AU` by the same rule, which is correct here because `en-AU` is the source Locale. A request for an unrelated language (say `fr`) matches nothing and takes the default Locale, which must be supplied as the `default` argument to `negotiate_languages` (both `i18n-embed` and `fluent-templates` pass their fallback language).

Because `en-XA` is a valid language identifier with region `XA`, the same step would also let an `en-XA` request fall through to `en-AU`. That is desirable: an incomplete pseudo file degrades to real English rather than blank text.

## 5. Pseudo Locale

`fluent-pseudo` (0.3.3) exposes `transform(s, flipped, elongated, accented)` and is installed with `FluentBundle::set_transform`, per its README. That lets `en-XA` be produced from the `en-AU` source at load time instead of maintaining a fourth `.ftl` set. With `i18n-embed`, `FluentLanguageLoader::with_bundles_mut` gives access to each bundle to apply the transform (the method exists in the source; applying `set_transform` through it was not tested here).

## 6. Recommendation

Build `lib-localisation` on **`i18n-embed` (`fluent-system` feature) with `i18n-embed-fl`**, keeping `fluent-langneg`, `unic-langid` and `fluent-pseudo` as direct dependencies of the library for negotiation and identifiers.

Why:

- It is the only option here that gives compile-time id and argument checking out of the box, pinned to the source Locale (`en-AU`), which is precisely what the map wants.
- Negotiation with `Filtering` gives the `en-GB -> en-AU -> source` chain without bespoke code, and the loader appends the fallback language to every selection.
- The loader is `Send + Sync` with `arc-swap` state, so one process-wide instance can serve the gpui and ratatui front ends, and Locale switching at runtime is a matter of calling `load_languages` or `select_languages_negotiate`.
- No UI framework dependency; the optional Locale-detection features are OS-only.

Tradeoffs:

- `fl!` checks only the fallback file and only literal ids; keep the cross-Locale test from section 3.
- Adds `rust-embed`, `arc-swap`, `parking_lot` and proc-macro crates, and an `i18n.toml`. Build-time and binary-size impact were not measured; a `cargo build --timings` and `cargo bloat` comparison against the alternative would settle it if it matters.
- Single `fallback_language` per loader. Fine for a two-level chain rooted at `en-AU`; a deeper hierarchy would need `select_languages` called with an explicit list.
- Upstream fluent-rs is stable but has not released since mid-2025 (langneg aside); a dependency on `fluent-bundle` directly carries the same exposure, so this is not a differentiator.

Runner-up: **`fluent-bundle` directly plus a small build-script generator**. Choose it if the team prefers to own about 100 lines of loader code, wants typed argument structs, or wants the smallest dependency tree. It is the natural escape hatch, since `i18n-embed` sits on the same bundles.

`fluent-templates` is a reasonable third choice with a good fallback story, but offers no compile-time checking and leans towards template-engine use that this project does not have.

## 7. Minimal sketch

Source file `i18n/en-AU/personal_ledger.ftl`:

```ftl
greeting = Welcome back, { $name }.
unread-transactions = { $count ->
    [one] { $count } transaction to review
   *[other] { $count } transactions to review
}
```

Library code (illustrative; not compiled here):

```rust
use i18n_embed::{fluent::{fluent_language_loader, FluentLanguageLoader}, LanguageLoader, DesktopLanguageRequester};
use i18n_embed_fl::fl;
use rust_embed::RustEmbed;
use unic_langid::langid;

#[derive(RustEmbed)]
#[folder = "i18n"]
struct Localisations;

pub fn loader() -> Result<FluentLanguageLoader, i18n_embed::I18nEmbedError> {
    // i18n.toml sets fallback_language = "en-AU".
    let loader = fluent_language_loader!();
    // Negotiates en-GB -> en-AU, then the fallback.
    loader.load_languages(&Localisations, &[langid!("en-GB"), langid!("en-AU")])?;
    Ok(loader)
}

pub fn summary(loader: &FluentLanguageLoader, name: &str, count: i64) -> (String, String) {
    (
        // Id and argument name checked against en-AU at compile time.
        fl!(loader, "greeting", name = name),
        // The plural branch is chosen by CLDR rules for the active Locale.
        fl!(loader, "unread-transactions", count = count),
    )
}
```

Direct `fluent-bundle` equivalent for the lookup with an argument and a plural:

```rust
use fluent_bundle::{concurrent::FluentBundle, FluentArgs, FluentResource};
use unic_langid::langid;

let res = FluentResource::try_new(ftl_source)?;
let mut bundle = FluentBundle::new_concurrent(vec![langid!("en-AU")]);
bundle.add_resource(res)?;

let mut args = FluentArgs::new();
args.set("count", 3);
let pattern = bundle.get_message("unread-transactions").and_then(|m| m.value());
let mut errors = vec![];
let text = pattern.map(|p| bundle.format_pattern(p, Some(&args), &mut errors));
```

Formatted values include Unicode isolating marks around placeables by default; the TUI may want `set_use_isolating(false)` (available on both `FluentBundle` and `FluentLanguageLoader`) so terminal column widths are not thrown off.

## 8. Open items

- Confirm binary-size and build-time deltas with a spike if either matters for the AppImage and dmg packaging.
- Confirm `with_bundles_mut` plus `set_transform` produces the `en-XA` pseudo output as intended.
- Decide whether `en-GB`/`en-US` files should be sparse overrides of `en-AU` (relies on the section 4 chain) or complete copies.

## Sources

- crates.io API metadata for every crate in section 1 (`https://crates.io/api/v1/crates/<name>`).
- `fluent-bundle` `Cargo.toml` and docs: `https://github.com/projectfluent/fluent-rs/blob/main/fluent-bundle/Cargo.toml`, `https://docs.rs/fluent-bundle/0.16.0/fluent_bundle/bundle/struct.FluentBundle.html`.
- `fluent-langneg` source and module docs: `https://github.com/projectfluent/fluent-langneg-rs/blob/main/src/negotiate/mod.rs`, `https://docs.rs/fluent-langneg/0.14.2/`.
- `fluent-pseudo` README: `https://github.com/projectfluent/fluent-rs/tree/main/fluent-pseudo`.
- `i18n-embed` README, `Cargo.toml` and `src/fluent.rs`: `https://github.com/kellpossible/cargo-i18n/tree/master/i18n-embed`; `https://docs.rs/i18n-embed/0.16.0/i18n_embed/fluent/struct.FluentLanguageLoader.html`; `https://docs.rs/i18n-embed-fl/latest/i18n_embed_fl/macro.fl.html`.
- `fluent-templates` README, `Cargo.toml` and loader source: `https://github.com/XAMPPRocky/fluent-templates`; `https://docs.rs/fluent-templates/0.15.1/`.
- `rust-i18n` README: `https://github.com/longbridge/rust-i18n`.
