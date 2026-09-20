# Localisation Message generator spike

Findings for [#230](https://github.com/IanTeda/Personal-Ledger/issues/230), a time-boxed spike on the [Localisation of the Desktop and TUI UX](https://github.com/IanTeda/Personal-Ledger/issues/211) map. It decides how typed Message accessors are generated and settles the risks that `docs/localisation-design.md` left open. All code was throwaway and lived outside the repo; nothing here is built in the workspace yet. The scaffold ticket ([#231](https://github.com/IanTeda/Personal-Ledger/issues/231)) is unblocked by this note.

## Recommendations

- **Generator:** write our own `build.rs` generator over `fluent-syntax`, in a small separate crate, `lib-locale-build`. Do not use `fluent-typed`. It cannot express sparse override Locales (the `en-GB` and `en-AU` layers), reports cross-Locale problems as warnings, and generates instance methods rather than the design's free functions over a process-wide loader.
- **Cross-crate:** the shared layer in `lib-locale` plus a bin-owned layer works. Each bin's `build.rs` runs the same generator with the runtime path `lib_locale::…`, and the runtime composes every layer into one `FluentBundle` per Locale. `i18n-embed-fl`'s `fl!` is not a usable fallback from a bin, because it validates ids only against its own crate's `i18n/`.
- **`en-XA`:** use `fluent_pseudo::transform_dom` with `with_markers` off, and add the outer `[ … ]` ourselves around the whole formatted Message. Plain `transform` mangles `<tag>` names, and `with_markers` brackets every text fragment rather than the whole Message. Private-use sentinels survive both, because they arrive as arguments.
- **Fallback:** the explicit chain `en-AU -> en-GB -> en-US` works with sparse Locales, by adding each Locale's resources to one bundle with `add_resource_overriding` in chain order (lowest priority first).
- **Costs:** Fluent plus ICU4X adds about 2.1 MB (Fluent) plus 1.3 MB (ICU4X, default data) to a stripped release binary, and 11 s plus 24 s to a clean release build. An `icu4x-datagen` trim to the four Locales cuts the ICU4X share from 1.29 MB to 0.31 MB and saves about 12 s of clean build. Trim, but as a follow-up step after the scaffold, not a blocker.

## Environment

- Machine: Intel Core i5-8250U (4 cores, 8 threads), 38 GB RAM, Linux 7.2.5-3-omarchy.
- Toolchain: `rustc` 1.98.0, `cargo` 1.98.0.
- Crates: `fluent-typed` 0.9.0, `fluent-bundle` 0.16.0, `fluent-syntax` 0.12.0, `fluent-langneg` 0.14.2, `fluent-pseudo` 0.3.3, `unic-langid` 0.9.6, `intl_pluralrules` 7.0.2, `i18n-embed` 0.16.0, `i18n-embed-fl` 0.10.1, `ratatui` 0.30.2, `icu_decimal` 2.3.0, `icu_datetime` 2.3.0, `icu_casemap` 2.3.0, `icu_experimental` 0.6.0, `icu_locale` 2.3.1, `icu_provider` 2.3.1, `fixed_decimal` 0.7.2, `icu4x-datagen` 2.3.0 (installed with `--features unstable`, which the currency markers need).
- Scratch projects, each with its own `CARGO_TARGET_DIR`: `a-ft` (`fluent-typed`), `b` (own generator, shared crate and bin), `c-pseudo`, `d-fl` (`fl!`), `e` (cost binary, compiled ICU4X data) and `e-trim` (cost binary, trimmed data).

## 1. Generator: `fluent-typed` or our own

### `fluent-typed` 0.9.0

What it does well, all verified with a `build.rs` using `BuildOptions::default().with_default_language("en-US").with_prefix("").without_bidi_isolation().with_lint_level(LintLevel::Strict)`:

- Kebab-case ids become snake_case methods (`accounts-add-submit` becomes `accounts_add_submit()`), and attributes become `<id>_<attr>` (`accounts-delete.title` becomes `accounts_delete_title()`).
- A selector over plural categories infers a number argument, typed `Into<FluentNumber>`, so a plain integer works (`accounts_delete_heading(3)` renders "Delete 3 accounts?"). A `(String)` comment types a string argument.
- Turning bidi isolation off is one option.

Where it falls short of the design:

- **Sparse Locales are not supported.** An accessor is generated only for a message present in every Locale with the same variables. With `en-US` complete and `en-GB`/`en-AU` holding only their overrides, every message that a sparse Locale lacks is skipped with a warning, and only `language-name` was generated. The workaround is to flatten the fallback chain into complete per-Locale `.ftl` files before `fluent-typed` sees them (verified: with hand-flattened files it generates everything), but the flattening step needs a Fluent parser anyway.
- **Cross-Locale problems are warnings, not errors.** A translation using a different variable name, or an id that is not in `en-US`, produces `cargo::warning=…` lines and the accessor is silently dropped, so the failure only shows up later as a missing method at a call site. The design's "every Locale is checked against `en-US`, run as `cargo test`" would need our own check regardless.
- **Method shape.** Accessors are methods on a per-language `L10nLanguage` value (`L10n::EnUS.load().accounts_add_submit()`), not free functions over a process-wide loader. The design's `msg::accounts_delete_title(count)` and its scoped thread-local test override do not map onto it without hand-written wrappers.
- **Unused-Message warnings rely on rustc's `dead_code`**, which is set to `allow` workspace-wide (`Cargo.toml`), so the warning would be silent. The design already expects the generator to emit its own report.
- **Generated code contains `.unwrap()` and `.expect()`** in every accessor and in `load()`, which conflicts with the workspace convention ([#256](https://github.com/IanTeda/Personal-Ledger/issues/256)), and by default it writes `src/l10n.rs` and `gen/translations.ftl` into the source tree. Pointing `with_output_file_path` at `OUT_DIR` works for the Rust file, but `gen/translations.ftl` is still written into the crate directory.
- **Attribute and id collisions** surface as a plain rustc `E0592` duplicate-definition error (`accounts-delete.title` collides with a message called `accounts-delete-title`). The design's id convention avoids this, but a generator-level check gives a clearer error.
- **Rich Messages** use its own `(Element)` annotation and segment structs, not the design's `<tag>` and sentinel convention.
- It is pre-1.0 and says its minor releases may break the API.

### Our own generator

A prototype over `fluent-syntax` 0.12 is 175 lines. It parses every `i18n/<locale>/*.ftl`, infers argument types (a selector whose keys are numbers or CLDR plural categories gives `i64`, otherwise `&str`), checks every non-source Locale against `en-US` for ids and exact variable sets, emits one free function per Message and attribute (`pub fn accounts_delete_heading(count: i64) -> String`), embeds each layer's files with `include_str!`, checks an id prefix (`desktop-`, `tui-`), detects accessor-name collisions, and prints a `cargo::warning` per unused Message by scanning the crate's sources. The runtime it calls (one process-wide `OnceLock`, a thread-local override for tests, one concurrent `FluentBundle` per Locale) is another 80 lines.

A production version adds: tag consistency checks (`<open>` in every Locale as in `en-US`), file and line numbers in errors, term and message reference validation, a more precise unused check (`msg::name(` call sites), and tests. That is an estimate of 400 to 500 lines for the generator and 200 to 300 for the runtime, against `fluent-typed`'s 5,195 lines that we would mostly not use.

### Is `lib-locale-build` needed

Yes, create it. A `build.rs` cannot use its own crate's normal dependencies, and a bin's `build.rs` needs the generator too. Making `lib-locale` a build-dependency would compile `fluent-bundle` and ICU4X for the host as well. The separate crate depends only on `fluent-syntax` (eight crates, including `thiserror`'s proc-macro dependencies). Feature-gating the generator inside `lib-locale` might also work, but it was not tested and is less clear.

## 2. Cross-crate layers

Verified with a `b-shared` crate (standing in for `lib-locale`, with `i18n/{en-US,en-GB,en-AU}`) and a `b-app` bin (with its own `i18n/{en-US,en-AU}`, ids prefixed `tui-`):

- The bin's `build.rs` runs the same generator with the runtime path `b_shared::runtime` and writes into its own `OUT_DIR`, included as `crate::msg`. The bin calls `msg::tui_palette_hint(1)` and `b_shared::msg::accounts_add_submit()` side by side.
- Each generated module exports a `LAYER` (the embedded files by Locale). `runtime::init(locale, &[shared::LAYER, msg::LAYER])` adds them, in chain order, to one bundle per Locale. A bin-only `en-AU` override ("1 hit" instead of "1 match") won over the bin's `en-US` text.
- Terms and messages from the shared layer are visible to bin messages at runtime, but the bin's build cannot see them, so the bin generator must not fail on a term or message reference it cannot resolve locally.
- Cross-layer id collisions cannot be seen at build time (the bin does not read the shared ids). `add_resource_overriding` would let a bin silently override a shared id, so the prefix rule plus a runtime check in `init` (report overlap) plus a test in each bin is recommended.
- `i18n-embed-fl`'s `fl!` works inside one crate against that crate's own `i18n/` assets (with its default isolating marks on), but a message that exists only in another crate's layer fails at compile time: "`message_id` of "accounts-add-submit" does not exist in the `fallback_language` ("en-US")". It also has no typed arguments, and each crate needs its own loader. It is not a fallback for shared ids. The untyped lookup the generated accessors call (`runtime::format(id, attr, args)`, which returns `⟦id⟧` on a miss) is the fallback for the rare dynamic id.

## 3. `en-XA`

`FluentBundle::set_transform` applies the transform to each text element of a Message, not to placeables and not to the whole Message. Results on `Press <open>open</open> to see { $key } details`, with `$key` a private-use sentinel argument (`U+E000` `0` `U+E001`):

| Transform | Result | Verdict |
| --- | --- | --- |
| `fluent_pseudo::transform(s, false, true)` | `Ƥřeeşş <ooƥeeƞ>ooƥeeƞ</ooƥeeƞ> ŧoo şeeee …` | mangles the tag names |
| `transform_dom(s, false, true, false)` | `Ƥřeeşş <open>ooƥeeƞ</open> ŧoo şeeee …` | keeps tags, mangles their content |
| `transform_dom(s, false, true, true)` | `[Ƥřeeşş <open>ooƥeeƞ</open> ŧoo şeeee ]0[ ḓeeŧaaiŀş]` | brackets each fragment around placeables |

- The sentinel code points survived every mode, both as an argument and when written literally in text.
- Recommendation: `transform_dom` with markers off, and wrap the whole formatted string as `[ … ]` in our `format` function. The Fluent tag names do not need to be emitted through a function or sentinel.
- Elongation grew the sample sentence `Delete the selected account` from 27 to 37 characters, close to the design's "about a third".
- A stray `<` in prose leaves the text up to the next `>` unmangled (`a < b and c > d`), so prose should avoid a bare `<`. The tag parser should treat unrecognised `<` as literal.
- ICU4X has no `en-XA` data, so the formatting layer should map `en-XA` to `en-US` before calling ICU4X. This was not exercised.

## 4. Fallback chain

The prototype builds one concurrent `FluentBundle` per Locale from the chain (`en-AU` adds `en-US`, then `en-GB`, then `en-AU`, each with `add_resource_overriding`), with the bundle's language set to the requested Locale (or `en-US` for `en-XA`) so plural rules match. Requested as `en-AU`:

- `accounts-add-submit` came from `en-AU` ("Add an account").
- `category-colour-label` came from `en-GB` ("Pick a colour for Food"), overriding `en-US` ("color").
- `only-in-us` (present only in `en-US`) resolved ("Only in en-US").
- A count of 2 selected the `[other]` plural variant from `en-US` ("Delete 2 accounts?").
- A thread-local override to `en-GB` rendered `en-GB` text while the process-wide loader stayed `en-AU`. It does not propagate to spawned threads.

`fluent-langneg` was only used in the cost binary. The design's rule (exact match wins, anything else becomes `en-US`, `en-XA` only by exact request) does not need it, so the scaffold can decide whether to keep the dependency.

## 5. Costs

The cost binary is a stand-in for `bin-tui`: `ratatui` 0.30.2 rendering a `Paragraph` into an off-screen `Buffer`, with no terminal backend, so the baseline is smaller than the real `bin-tui`. The deltas are the meaningful figures. The binary calls Fluent (`fluent-bundle`, `fluent-langneg`, `fluent-pseudo`, the prototype runtime), `icu_decimal`, `icu_datetime`, `icu_casemap` and `icu_experimental`'s `CurrencyFormatter`, all for `en-AU`, so nothing is optimised away. Its output confirmed the formatters work: `1,234,567.89`, `ACCOUNTS`, `20 Sept 2026` (ICU4X's `en-AU` abbreviates September as "Sept") and `US$1,234,567.89`.

Variants: **v0** baseline (`ratatui` only), **v1** plus Fluent, **v2** plus ICU4X with its default compiled data, **v3** plus ICU4X with data generated by `icu4x-datagen` for `en-US`, `en-GB` and `en-AU` and only the markers the binary uses.

Release profile (defaults), each variant built from a clean target directory. Incremental is after `touch src/main.rs`. Sizes are after `strip`.

| Variant | Clean | Incremental | Stripped size | Added over previous |
| --- | --- | --- | --- | --- |
| v0 baseline | 11.5 s | 0.2 s | 495,784 B | |
| v1 + Fluent | 22.4 s | 0.6 s | 2,588,432 B | +2.09 MB |
| v2 + ICU4X (default data) | 46.5 s | 0.9 s | 3,882,272 B | +1.29 MB |
| v3 + ICU4X (trimmed data) | 34.9 s | 0.9 s | 2,899,824 B | +0.31 MB |

Size-tuned release profile (`lto = true`, `codegen-units = 1`, `opt-level = "s"`, `strip = true`, `panic = "abort"`), clean builds:

| Variant | Clean | Size |
| --- | --- | --- |
| v0 baseline | 13.6 s | 382,128 B |
| v1 + Fluent | 27.8 s | 1,787,640 B |
| v2 + ICU4X (default data) | 48.7 s | 2,933,304 B |
| v3 + ICU4X (trimmed data) | 41.5 s | 1,973,560 B |

Dev profile (unoptimised, full debuginfo): clean 9.4 s, 12.4 s, 30.1 s and 22.7 s for v0 to v3, incremental 0.2 to 0.5 s, and unstripped binaries of 14.7 MB, 37.9 MB, 68.8 MB and 65.9 MB.

- Fluent is the larger fixed cost (about 1.4 to 2.1 MB), because `fluent-bundle` pulls in `intl_pluralrules` and its memoiser.
- The trim saves about 0.98 MB (25% of the total) and about 12 s of a clean release build, in either profile. After the trim, ICU4X adds only about 0.2 to 0.3 MB.
- The trim is a real workflow: `icu4x-datagen --markers-for-bin <release binary> --locales en-US en-GB en-AU --format baked --use-separate-crates --out data` took 10.6 s (installing the tool takes about 2.5 minutes), and downloads CLDR data at generation time. The output (388 KB of Rust) is wrapped in a small crate (`include!("../data/mod.rs")`, `pub struct Baked; impl_data_provider!(Baked);`, plus `icu_locale_fallback` with `compiled_data`, `icu_pattern`, `zerovec` and `extern crate alloc`) and used through the `_unstable` constructors (`DecimalFormatter::try_new_unstable(&Baked, …)`). Output was identical to the untrimmed build.
- The trimmed data covers only the markers that binary used. A new formatter call (another date field set, compact currency) needs regeneration. A provider that lacks a marker should fail to compile, because `Baked` only implements the included markers, but this was not verified.
- `icu_experimental` 0.6.0 churn is real and shows in the API: there is no `CurrencyFormatter::try_new`; the currency is fixed at construction (`try_new_symbol(prefs, currency!("USD"), options)`), so a formatter is built per Locale and Unit; and the crate carries a `TODO(#8146)` about `FixedDecimal` being the wrong input type. The design's plan to wrap and pin it stands.

## What this did not settle

- A real `bin-tui` or `bin-desktop` build. The cost binary is a stand-in, and incremental numbers on a tiny crate say nothing about a bin dominated by its own code.
- The rich-Message segment parser and sentinel splitter (only `en-XA` interplay was tested), and the `⟦id⟧` miss path (implemented in the prototype, not exercised).
- Terms, functions other than plural selectors, and `Money` to `fixed_decimal::Decimal` conversion.
- Whether a missing ICU4X marker fails at compile time or runtime with a trimmed provider.
- Gpui-side `set_locale` and `ratatui` width behaviour, covered by separate research.

## Changes to the scaffold ticket (#231)

- Create `lib-locale-build` (depends on `fluent-syntax` only) and use it from `lib-locale/build.rs` and each bin's `build.rs`. Do not add `fluent-typed`.
- The generator emits free functions taking `i64` for counts and `&str` otherwise, embeds each layer's files, and exposes `LAYER`; the runtime composes layers into one bundle per Locale with `add_resource_overriding`.
- The consistency checks are ordinary tests that fail (not `cargo::warning`), plus the generator's own errors for ids, variables and tags.
- `en-XA` uses `transform_dom` with markers off, plus our own outer brackets.
- Emit an unused-Message report from the generator, not from rustc.
- ICU4X data trimming is a later, separate ticket.
