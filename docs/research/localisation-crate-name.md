# Research: Naming precedents for the shared localisation crate

**Question.** GitHub issue #223 (child of map #211) asks what the shared localisation crate should be called. `lib-localisation` is the working name across the map but is not settled. This note gathers the facts a naming decision needs: naming conventions across the Rust ecosystem, existing crates.io names that could collide or confuse, how this workspace's existing crate names read, and how each candidate would look in a config section, environment variable, `use` path and macro. It recommends nothing binding; the decision belongs to the map.

Research was done by querying the crates.io API directly (versions, downloads, descriptions, dependency lists), unpacking the published `gpui-component` 0.6.4 source tarball, reading the READMEs of `fluent-rs`, `cargo-i18n`/`i18n-embed`, `rust-i18n` and ICU4X, the Rust API Guidelines naming chapter, and the Firefox Source Docs localisation page. Download counts and versions are as of 2026-09-20.

## 1. Conventions across the Rust ecosystem

The Rust API Guidelines (`https://raw.githubusercontent.com/rust-lang/api-guidelines/master/src/naming.md`) mark crate-name casing as "unclear" (linked to api-guidelines issue #29) and say only that crate names should not use `-rs` or `-rust` as a prefix or suffix. Macros are `snake_case!`. There is no prescribed vocabulary for localisation, so precedent comes from projects.

- **`i18n` is the dominant crate-name token for runtime translation lookup.** `rust-i18n` (4.2.2, 3.47M downloads, "a `t!` macro") and its `rust-i18n-macro`, `i18n-embed` (0.16.0, 6.36M) and `i18n-embed-fl` (6.24M, provides `fl!`), `cargo-i18n` and `i18n-build`, plus framework glue such as `leptos_i18n` and `dioxus-i18n`. The macro in these crates is short (`t!`, `fl!`, `tr!`) and does not carry the crate token.
- **`fluent-*` is the Mozilla/Project Fluent family, named after the format rather than the concern.** The `fluent-rs` README lists `fluent`, `fluent-bundle`, `fluent-fallback` ("locale bundles and runtime localization lifecycle"), `fluent-resmgr`, `fluent-syntax`, `fluent-pseudo`, and `fluent-langneg` ("language and locale negotiation", 18.4M downloads) is a sibling. Mozilla's own docs (`https://firefox-source-docs.mozilla.org/l10n/index.html`) spell the concern "Localization" and note the `l10n` abbreviation, with `l10n` as the directory/tooling name inside Gecko.
- **`icu*` / `icu_locale*` is ICU4X's scheme (Unicode Consortium).** The umbrella crate is `icu` (2.3.1); component crates are named by capability: `icu_locale` and `icu_locale_core` ("Unicode Language and Locale Identifiers", 402M downloads), `icu_provider`, and so on. ICU4X handles locale data and formatting (dates, numbers, plurals), not message catalogues. Its README uses `use icu::locale::locale;`, so `locale` is the module name for identifiers.
- **`locale` is a capability-noun for identifier handling.** Used for module names inside ICU4X, `fluent-langneg`, and `sys-locale` (33M downloads, "obtain the active system locale"). The crates.io crate `locale` (0.2.2, last released 2017-05-26, 1.05M downloads, "Library for basic localisation") is abandoned but still occupies the name.
- **`intl` and `translate` as whole-concern names are rare.** `intl` (0.6.3, 14k downloads) is a `no_std` ICU analogue. Nothing prominent uses `translate` for a crate name.
- **`l10n` as a crate name has one occupant.** `l10n` (0.1.3, 5k downloads, "Opinionated localization library built upon fluent-bundle").
- **Zed and gpui-component.** Zed's `crates/gpui/Cargo.toml` has no i18n or locale dependency, and Zed has open work on the topic (issue #7409 "Localization and support for different locales", open; #7433 "Add Localization Crate with i18n standard implementation" and #62027 "Add user interface localization and a Simplified Chinese catalog", both closed; titles from the GitHub issue search API, outcomes not read) but no adopted crate name. `gpui-component` 0.6.4 takes a hard (non-optional) dependency on `rust-i18n ^4.2.0` and, in `src/lib.rs`, calls `rust_i18n::i18n!("locales", fallback = "en");` and exposes `gpui_component::locale()` and `gpui_component::set_locale(&str)`, both thin wrappers over `rust_i18n::locale()` and `rust_i18n::set_locale()`. Its catalogue is a single `locales/ui.yml`; components call `rust_i18n::t!("Select.placeholder")`-style keys.

Spelling: crate-name tokens in the ecosystem use the American `localization` where the full word appears (`fluent-fallback` description, `i18n-embed` description, `l10n`, Mozilla docs, ICU4X). The British/Australian `localisation` appears in only a few descriptions (`tr` "tr! macro for localisation", `locale` "basic localisation"). crates.io returned no crate named `localisation` and none named `lib-localisation`; `localization` exists (0.1.5, 6k downloads, "`t!` macro, the easiest way") and `i18n` exists (0.1.0, 64 downloads, Dioxus compile-time i18n).

## 2. Collision and confusion risks

The workspace crate is unpublished, so crates.io name availability matters only for confusion and a possible later publish. The real collision surface is the package name `lib_<name>`, its `use` path, and any re-exported macros.

- **crates.io names.** `lib-localisation`, `lib_localisation` and `localisation` are unclaimed. `localization`, `i18n`, `l10n`, `locale` and `intl` are all taken, so a bare crate of those names could not be published; irrelevant for `lib-*` prefixed names.
- **Macro `t!` clashes if glob-imported.** `gpui-component` and `rust-i18n` already export `t!` (`rust_i18n::t`). If a workspace crate re-exports its own `t!` and a binary also writes `use rust_i18n::*`, the names collide; `gpui-component` itself only imports `rust_i18n::t` explicitly. Re-exporting `rust-i18n`'s own `t!` would avoid two macros of the same name.
- **`i18n!` is per-crate state.** `rust_i18n::i18n!("locales", ...)` embeds the catalogue and generates crate-local items (`gpui-component` calls it once at its crate root). The workspace crate can make its own call; it does not merge with `gpui-component`'s embedded `locales/ui.yml`, so the two catalogues stay separate. Note `gpui_component::set_locale` delegates to `rust_i18n::set_locale`, so a workspace crate that also uses `rust-i18n` shares the same locale setter; a workspace crate built on a different backend (for example Fluent) would need to call both to keep component chrome and app text in step. This is worth confirming in the follow-up ticket (#224) rather than assuming.
- **Name `locale` in `use` paths.** `gpui_component::locale()` is an existing function; a workspace module or re-export named `locale` would sit beside it but not clash unless both are glob-imported.
- **Confusion with the `locale` crate and `sys-locale`.** A candidate containing only `locale` reads as identifier/system-locale handling (what ICU4X and `sys-locale` do), not message translation.

## 3. How this workspace's names read

Existing library crates in this checkout are `lib-config`, `lib-core`, `lib-database`, `lib-rpc` and `lib-telemetry`; the issue also lists `lib-omarchy`, `lib-tracing` and `lib-mcp` from other branches, which do not exist in this worktree's base commit and were not verified here. The `CLAUDE.md` convention is a `lib-` prefix (package `lib_x`) plus a single noun for the concern: `config`, `core`, `database`, `rpc`, `telemetry`. There are no verbs (`configure`, `translate`) and no format or vendor names in the crate name (no `lib-fluent`, `lib-sqlx`). Where a word varies, CLAUDE.md requires Australian English for comments and rustdoc; the table below applies that to the name.

Two tokens of this workspace are worth noting. Abbreviations already appear (`rpc`), so a numeronym like `i18n` would not be alien, but `rpc` is a universally recognised initialism whereas `lib-i18n` is a numeronym. All other names are full words. That favours the full word `localisation`, and its Australian spelling is consistent with the CLAUDE.md rule.

## 4. Candidates and how each would read

Config section and env var follow `lib-config`: INI sections lower-cased, `PERSONAL_LEDGER_*` with double-underscore nesting (for example `PERSONAL_LEDGER_TELEMETRY__TELEMETRY_LEVEL`).

| Candidate | Package / `use` path | Config section | Env var | Macro |
| --- | --- | --- | --- | --- |
| `lib-localisation` | `lib_localisation` | `[localisation]` | `PERSONAL_LEDGER_LOCALISATION__LANGUAGE` | `t!` (short, no crate token) |
| `lib-i18n` | `lib_i18n` | `[i18n]` | `PERSONAL_LEDGER_I18N__LANGUAGE` | `t!` |
| `lib-l10n` | `lib_l10n` | `[l10n]` | `PERSONAL_LEDGER_L10N__LANGUAGE` | `t!` |
| `lib-locale` | `lib_locale` | `[locale]` | `PERSONAL_LEDGER_LOCALE__LANGUAGE` | `t!` |
| `lib-intl` | `lib_intl` | `[intl]` | `PERSONAL_LEDGER_INTL__LANGUAGE` | `t!` |

Tradeoffs:

- **`lib-localisation`.** Matches the workspace pattern (full-word noun, Australian spelling) and the working name across the map. Longest to type in `use lib_localisation::t;` and env vars (`PERSONAL_LEDGER_LOCALISATION__...`), but tab-completion and a single `use` line absorb this. The Australian spelling differs from every upstream crate and doc (`localization`), so searching crates.io or docs.rs for related material needs the American spelling. No crates.io collision.
- **`lib-i18n`.** Shortest and most searchable; matches `rust-i18n`, `i18n-embed`, `cargo-i18n`, the `i18n!` macro name, and gpui-component's backend. Cost: a numeronym in a workspace of full words; the crate would also share its token with the `i18n!` macro it may call, making `lib_i18n::i18n!` stutter.
- **`lib-l10n`.** Mozilla's own tooling token. Only one crates.io crate uses it (`l10n`, 5k downloads), so it is the least recognisable to Rust readers, and it is the most likely to be mistaken for a format-specific (Fluent) crate.
- **`lib-locale`.** Short and a full word, but the ecosystem uses it for identifier and system-locale handling (`icu_locale`, `sys-locale`, `fluent-langneg`), so it under-describes a crate that also owns message catalogues, formatting and config. It would also sit beside `gpui_component::locale()`.
- **`lib-intl`.** Matches the JavaScript `Intl` and ICU4X's internationalisation-primitives space (the crates.io `intl` is a no-std ICU analogue); implies formatting of dates and numbers rather than translated UI strings. Unfamiliar in this workspace's vocabulary.

Not shortlisted: `lib-translate` (a verb, against the workspace's noun-only pattern) and `lib-fluent` (names an implementation choice, against the pattern of naming the concern; it would also force a rename if the backend switched to `rust-i18n`).

## 5. Findings summary

- No Rust API Guidelines rule constrains the choice; crate casing is "unclear" and only `-rs`/`-rust` affixes are discouraged.
- The ecosystem majority uses `i18n` in crate names and short macros (`t!`, `fl!`, `tr!`), with the American `localization` spelling wherever the full word appears.
- `lib-localisation` and `lib_localisation` are unclaimed on crates.io; `localization`, `i18n`, `l10n`, `locale` and `intl` are taken.
- `gpui-component` hard-depends on `rust-i18n` and exposes `locale()`/`set_locale()` over it, so a `t!` re-export and the process locale are the clash points to design around, not the crate name itself.
- The workspace pattern (full-word noun, Australian spelling, `lib-` prefix) points to `lib-localisation`; `lib-i18n` is the strongest alternative on ecosystem familiarity.
