# Localisation design

The developer-facing design for localisation. Looking for the user guide (how to change your Locale, what changes)? See [Localisation](localisation.md).

How Personal Ledger presents its interface in a Locale: which Messages the desktop and TUI show, and how numbers, dates and currency amounts are formatted. This page records the decisions reached on the [Localisation of the Desktop and TUI UX](https://github.com/IanTeda/Personal-Ledger/issues/211) Wayfinder map; each decision's detail lives on its ticket, and the hard-to-reverse parts are in [ADR-0021](adr/0021-locale-owns-formatting-and-replaces-number-and-date-preferences.md). The terms Locale, Message and Catalogue are defined in `CONTEXT.md`.

## Status

Built. The shared crate, `lib-locale`, is in the workspace and both bins are migrated. The crate landed ticket by ticket — the Locale set, Catalogues, loader and generated `msg::` accessors ([#231](https://github.com/IanTeda/Personal-Ledger/issues/231)), formatting ([#232](https://github.com/IanTeda/Personal-Ledger/issues/232)), typed date input ([#233](https://github.com/IanTeda/Personal-Ledger/issues/233)), the `Label` trait ([#234](https://github.com/IanTeda/Personal-Ledger/issues/234)) and rich Messages ([#235](https://github.com/IanTeda/Personal-Ledger/issues/235)) — followed by the surface-text migration of `bin-desktop` (shell chrome, Accounts, Transactions, Settings, Dashboard and Explorer) and `bin-tui` (Accounts, Categories, Payees, Tags, Units and Settings, [#247](https://github.com/IanTeda/Personal-Ledger/issues/247)–[#252](https://github.com/IanTeda/Personal-Ledger/issues/252)). New UI text goes in the `en-US` Catalogue and is read through a generated `msg::` accessor — don't add a hardcoded literal. The items still outstanding are listed under [Open items](#open-items).

## Locales

- **`en-US`** is the source Locale: every Message is first written in it, and it is the only complete Catalogue. A missing Message there is an error.
- **`en-GB`** and **`en-AU`** are sparse override layers holding only the Messages that differ from the Locale they fall back to. The fallback chain is set explicitly in the crate, not left to negotiation: `en-AU` falls back to `en-GB`, which falls back to `en-US`. Australian spelling matches `en-GB` far more than `en-US`, so `en-AU` holds only what differs from `en-GB`.
- **`en-XA`** is a pseudo-Locale used to find untranslated text and layout overflow. It is generated from `en-US` at load time (accents and roughly a third extra length, with bracket delimiters), so no fourth Catalogue file exists. It is present in every build, never listed in Settings, and reachable only through the `locale` configuration. Its formatting falls back to `en-US`, because ICU4X has no `en-XA` data.
- Locales beyond these four, right-to-left scripts and non-Latin fonts are out of scope. A future real translation (for example `fr`) must be a complete Catalogue, and a missing Message there is a warning rather than an error.

## How the Locale is chosen

The Locale is Configuration, not a Preference. It is per-Client, never synced, needs no database read, and changes only by editing the configuration and restarting the Client. There is no in-app switch and no runtime Locale change to propagate.

`lib-config` resolves the requested Locale, from lowest to highest precedence:

- A `DEFAULT_LOCALE` constant, `en-US`.
- The operating system's locale, detected with `sys-locale`.
- The `locale` key in the `[Personal-Ledger]` section, which follows `lib-config`'s existing layers: config files, then the `PERSONAL_LEDGER_PERSONAL_LEDGER__LOCALE` environment variable, then the `--locale <TAG>` (no short flag) command-line flag.

The layers are read into `locale: Option<String>` (absent means unset) and resolved in a step after the build: the config value if present (source `Config`), else the detected system locale (source `System`), else `DEFAULT_LOCALE` (source `Default`). There is no `system` keyword: an absent key means the detected system locale stands. The value is validated as a BCP-47 tag with `unic-langid` internally and stored as a plain canonical-cased string, and the `LocaleSource` lets the Settings page show the effective Locale read-only with where it came from. A malformed value from a file, environment variable or flag is a startup error (`InvalidLocale`). A valid but unsupported tag passes through, because `lib-config` does not know the supported set. An unreadable system value (`C`, `POSIX`) is ignored quietly. The Sync Server skips detection and never reads the field.

The double-underscore environment overrides work as documented: `lib-config` sets `prefix_separator("_")` and `separator("__")` explicitly on the `config` crate's `Environment` source, because `config` otherwise reuses one separator for both and nested keys never resolve.

`lib-locale` then negotiates that string against the supported set: an exact match wins, anything else (`en-NZ`, `fr-FR`, `C`, `POSIX`) becomes `en-US`, and `en-XA` is chosen only by an exact request. Each bin's `main` calls `Config::parse` and then `lib_locale::init`, which sets the process-wide loader once. Settings has no control to change the Locale.

## Formatting

The Locale owns number, date and currency formatting, replacing the earlier `NumberFormat` and `DateFormat` Preferences (ADR-0021). The two definitions that had already drifted apart, in `lib-core` and in `bin-desktop`, collapse into `lib-locale`.

- **Numbers:** the number-separator Preference is deleted, because `en-US`, `en-GB` and `en-AU` all use `1,234.56`.
- **Dates:** the date Preference becomes a nullable, synced date style (short, medium, long or ISO), persisted as `DateStyle` in `lib-core`. No value means the Locale's default, and ISO overrides the Locale for both display and typed input. The date style still changes live, because it only changes formatting output.
- **Typed date input:** follows the Locale's short form (read from the resolved ICU4X pattern), always also accepts ISO `YYYY-MM-DD`, and accepts `today`, `yesterday` and `tomorrow`. The accepted words come from `date-word-*` Messages, with the English words always accepted. Two-digit years are accepted and year-less input is opt-in per call site. ICU4X does not parse, so this is a small parser in `lib-locale` over `NaiveDate::from_ymd_opt`.
- **Currency:** the Unit decides what is shown (its code and fraction digits) and the Locale decides how (separators, symbol placement, disambiguation such as `A$` for an `en-US` user). Fiat Units go through the ICU4X currency formatter, wrapped and pinned because `icu_experimental` is not semver-stable. Non-fiat Units (tickers, bitcoin) format the quantity as a decimal and place the Unit code through a Message.
- **Exactness:** amounts stay `Money` (`BigDecimal`), converted with `to_plain_string()` and never through `f64`. Fluent's own `NUMBER()` and `DATETIME()` are not used, because in the Rust implementation they do no locale formatting.

## Messages

- **What is a Message:** labels, headings, prose, validation and status text, command descriptions, argument placeholder words and example prose, hints, the labels of enumerated domain values, and user-reachable error text.
- **What is not:** command names and syntax (`accounts new`, `[into <acct>]`), which are stable English ids typed by the user and dispatched on an id, never on display text; mock and seed data; element ids, paths, config keys and URLs; tracing, panic and `expect` text; `sync-server` output; CLI `--help`; and temporary `not yet built` placeholders, which are deleted as features land.
- **Domain vocabulary:** each variant of `AccountType`, `TransactionStatus`, `CategoryTypes`, `UnitKind` and `BudgetPeriod`, plus Flagged, has one Message with one canonical translation per Locale, shared by both bins. Stored tokens (`as_str()`) stay stable and are never displayed. A `Label` trait in `lib-locale` provides them, so a call site reads `account_type.label()`, and each implementation is an exhaustive `match`, so a new variant fails to compile until its Message exists.
- **Errors:** libraries keep returning typed `thiserror` errors with English `Display` for developers and logs. The bin maps each user-reachable variant (about 10 to 15) to a Message. Developer-facing wrapper errors are not Messages and are never spliced into a localised sentence through `err.to_string()`.
- **Plurals:** every count-plus-noun is a Fluent plural selector, even where English needs only one/other, and stems plus suffixes (`categor{y,ies}`) are banned.
- **Glyphs:** the label is the Message and the glyph is a separate argument, because glyph style is a Display Preference. The join is one Message with two variables.
- **Casing and abbreviations:** a Message is written once in natural sentence case. Upper-casing is done by the renderer through a Locale-aware helper. Abbreviations are not the default: the source Message uses the full word, and a `.short` variant exists only where layout needs it.
- **Sentences:** one Message per complete sentence or paragraph, never split for styling. Hard-wrapped literals are rejoined, and wrapping is the renderer's job.
- **Seeded rows:** system-seeded rows that ship into synced data (such as `No institution`) store a stable key and render the Message at display time, so Clients in different Locales each show their own.

## Looking up a Message

Call sites use **generated typed accessor functions** with typed arguments, checked against `en-US`, not string ids and not a macro. A plural count is a typed integer and key tokens are typed arguments.

```rust
// bin-desktop
div().child(msg::accounts_delete_title(count))

// bin-tui
Line::from(msg::accounts_delete_title(count))
```

- **Module:** `lib_locale::msg::…` for the shared layer, and each bin's own `crate::msg::…` for its bin-only Messages. Bin-only ids carry `desktop-` or `tui-` prefixes, so the two modules never share a function name.
- **Ids:** Fluent ids are kebab-case, `<area>-<thing>-<role>`, named for their role rather than their English text (`accounts-add-submit`). Related Messages such as a dialog's title, body and buttons use Fluent attributes on one id (`accounts-delete.title`). The generated accessor is the same name in snake_case. Shared vocabulary uses fixed families (`account-kind-credit-card`, `transaction-status-open`).
- **Loader:** one process-wide loader, set once at startup, so views need no `cx` or handle plumbing and lookups work in both `render` and `draw`. Tests get a scoped, thread-local override so parallel tests can use different Locales.
- **Arguments:** only counts go in as numbers, for plural selectors. Money, dates and other formatted values arrive as strings from the formatting layer. Unicode isolating marks around placeables are switched off globally.
- **Misses:** a missing Message never panics. It resolves through the fallback chain to `en-US`, and an id missing everywhere (possible only for a dynamic id) returns the id wrapped as `⟦id⟧` in debug builds and plain in release, with one `tracing::warn!`.
- **Styled and clickable spans:** key tokens and nouns are normal string arguments passed as sentinels (a private-use code point pair carrying an index), and the formatted text is split so the caller styles each token. A translated styled span (the clickable `open` in a sentence) uses a lightweight tag convention, `<open>open</open>`, parsed into segments with an optional tag. Both return segments with no UI types, each bin maps tags to its own styling or click handler, arguments are escaped so a value cannot forge a tag, and a test requires each Locale to use the same tags as `en-US`.

## Catalogues and delivery

- **Embedded at compile time.** The Windows TUI is a single portable `.exe` and the AppImage is a single file, all four Locales are shipped with the app, and `bin-desktop` already embeds its icons with `include_bytes!`. Loading user-supplied Catalogues from disk is not planned.
- **Layers:** a shared layer in `lib-locale` (navigation, domain vocabulary, column labels, common dialogs, formatting Messages) plus a layer each bin owns for its own screens. The loader adds the layers as resources to one bundle per Locale, and each bin's `build.rs` generates its own accessors with the same checks.
- **Files:** `crates/libs/lib-locale/i18n/<locale>/<area>.ftl` for the shared layer, and `crates/bins/bin-desktop/i18n/<locale>/*.ftl` and `crates/bins/bin-tui/i18n/<locale>/*.ftl` for each bin's own.
- **Checks**, run as ordinary `cargo test`: every id in every Locale exists in `en-US`, variables and tags match, `en-US` has no missing Message a bin references, and an `en-XA` smoke test renders every Message without a panic or a literal id. Unused Messages are a warning from the generator, which emits its own report because the workspace allows `dead_code`.
- **Adding a screen:** add its Messages to `en-US` only. `en-GB` and `en-AU` inherit until they need to differ, and `en-XA` is automatic.

## Crate structure

```text
crates/libs/lib-locale/
  build.rs             generates msg/ accessors
  i18n/<locale>/*.ftl  the shared layer
  src/
    locale.rs          supported set, negotiation, fallback chain
    catalogue/         embedded layers, bundles, pseudo transform, loader, test override
    msg/               generated shared accessors
    rich.rs            sentinel splitter and tag parser, no UI types
    format/            number, date, currency, input (typed dates), casing
    vocabulary.rs      the Label trait for lib-core enums
    error.rs
  tests/               cross-Locale check, en-XA smoke test
```

- `lib-locale` depends on `lib-core`, never the reverse, so `lib-core` stays pure with no I/O and no ICU.
- `lib-config` depends on neither `lib-locale` nor ICU4X, and `lib-locale` does not depend on `lib-config`. `bin-sync-server` never depends on `lib-locale`.
- The generator is a small separate crate, `lib-locale-build`, depending only on `fluent-syntax` and used by `lib-locale`'s and each bin's `build.rs`, because a `build.rs` cannot use its own crate's dependencies and a build-dependency on `lib-locale` would compile Fluent and ICU4X for the host. The spike ([research note](research/localisation-message-generator.md)) chose our own generator over `fluent-typed`, so the crate is created.
- No `fluent-bundle`, `icu` or `unic-langid` type appears in any public signature, so a dependency can change without touching the bins.
- **Public surface:** `init(&str) -> Locale`, `Locale`, the generated `msg::…` accessors, the rich segment types, `format_money(&Money, &Unit)`, `format_date(NaiveDate, DateStyle)`, `format_number`, `parse_date(&str, today)`, a Locale-aware `upper`, the `Label` trait, and a `with_locale(...)` test helper.
- **Dependencies:** `fluent-bundle`, `fluent-langneg`, `fluent-pseudo` (and `fluent-syntax`, in `lib-locale-build` only), `unic-langid`, `icu` (`icu_decimal`, `icu_datetime`, `icu_casemap`, and a pinned `icu_experimental` for currency), and `sys-locale` in `lib-config`.

## The name

The crate is `lib-locale` (crate `lib_locale`), chosen over `lib-localisation` and `lib-i18n`. It under-describes a crate that also owns Catalogues and formatting, so its rustdoc must state the whole scope up front, and `gpui_component::locale()` must be imported explicitly rather than glob-imported. Identifiers and Message ids use Australian spelling (`Catalogue`, `colour`); Message text follows its Locale, so `en-US` writes "color" and `en-GB` and `en-AU` override it as "colour".

## Interaction with the UI stacks

- `gpui-component` ships its own `rust-i18n` (English, Simplified and Traditional Chinese, and a partial Italian). The Locale is a process-wide static, so the desktop calls `gpui_component::set_locale` once at startup with the mapped tag (any `en-*` maps to `en`). Its calendar month and weekday names, the input context menu and dock strings cannot be overridden from outside, so they stay English or are avoided. Its `Dialog` button labels and the placeholders on `Select`, `DatePicker`, `List` and `Input` are overridden at the call site from `lib-locale` Messages.
- `gpui` has no Locale concept. Because the Locale never changes at runtime, no change signal or `refresh_windows` is needed.
- `ratatui` measures width in terminal cells with `unicode-width`. Accented Latin is one cell wide, but a longer Message is clipped, not wrapped, unless the widget wraps. Any width arithmetic must use `UnicodeWidthStr::width`, never `str::len()` or `chars().count()`, and fixed-width columns and one-line hints need generous constraints or explicit truncation so `en-XA` passes cleanly.

## Open items

- Settled by the generator spike ([note](research/localisation-message-generator.md)):
  - We write our own generator over `fluent-syntax`, not `fluent-typed`, which cannot express sparse override Locales, reports cross-Locale problems as warnings, and generates instance methods rather than free functions over a process-wide loader. Accessors take `i64` for counts and `&str` otherwise, and the kebab-case to snake_case conversion works, with a generator check for accessor-name collisions (an attribute `accounts-delete.title` collides with a message `accounts-delete-title`).
  - A bin generates its own layer from `bin-*/i18n/` with the same generator, and the runtime composes every layer into one bundle per Locale with `add_resource_overriding` in fallback-chain order. `fl!` from `i18n-embed-fl` is not a fallback, because it validates ids only against its own crate's assets. The bin build cannot see the shared layer's terms and ids, so it must not fail on an unresolved reference, and cross-layer id collisions are caught by the `desktop-`/`tui-` prefixes, a check at `init` and a test.
  - `en-XA` uses `fluent_pseudo::transform_dom` with markers off, because plain `transform` mangles `<tag>` names and its markers bracket each text fragment; the generated outer `[ … ]` is added around the whole Message. The private-use sentinels survive the transform.
  - The chain `en-AU -> en-GB -> en-US` works with sparse Locales.
- Measured on a `ratatui` stand-in, for a stripped release binary: Fluent adds about 2.1 MB (1.4 MB with a size-tuned profile) and 11 s of clean build, and ICU4X with its default data adds about 1.3 MB and 24 s. An `icu4x-datagen` trim to `en-US`, `en-GB` and `en-AU` cuts the ICU4X share to about 0.3 MB and saves about 12 s. The trim is a separate, later ticket, and must be regenerated when a new ICU4X marker is used.
- Since landed: the per-screen string migration, the Preference migration (the number-separator column is gone and `date_format` is now the nullable `date_style`), and the TUI palette's move from dispatch-on-display-text to dispatch-on-id.
- Still outstanding: the `icu4x-datagen` trim above; the seeded-row keys (the `institution-none` Message exists, but there is no Institutions table yet and `bin-desktop` still holds a `NO_INSTITUTION` literal); translation workflow; per-Locale command aliases; user-supplied Catalogues; and mapping Commonwealth variants (`en-NZ`, `en-IN`) to `en-GB`.
- The cost figures come from a stand-in binary, not a real `bin-tui` or `bin-desktop` build. Both bins now carry `lib-locale`, so the figures can be re-measured for real.

## Research

The generator spike's findings are in [Localisation Message generator spike](research/localisation-message-generator.md). The other findings behind these decisions are Markdown notes on throwaway `research/localisation-*` branches (they are not merged): the Fluent binding, formatting, gpui and ratatui behaviour, the hardcoded-text inventory, crate naming precedents, date input parsing, and the confirmation of Fluent against alternative message systems (which found no blocker: MessageFormat 2's Rust runtime is not yet usable, and the gaps in Fluent are ones the shared crate would add anyway).
