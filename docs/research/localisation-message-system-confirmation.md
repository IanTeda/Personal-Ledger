# Research: Confirming Fluent against alternative message systems

**Question.** [Issue #226](https://github.com/IanTeda/Personal-Ledger/issues/226), a child of the Localisation map ([#211](https://github.com/IanTeda/Personal-Ledger/issues/211)): Ian suspects Fluent is missing features the project needs. Enumerate Fluent's real gaps against the project's needs, evaluate the alternatives against the same needs, separate blockers from work the shared `lib-localisation` crate would add anyway, and recommend staying on Fluent or switching. The earlier notes took Fluent as fixed (`localisation-fluent-binding.md`, which compared bindings), settled ICU4X as the formatting engine (`localisation-formatting.md`) and mapped the gpui, gpui-component and ratatui constraints (`localisation-ui-stacks.md`); none of that is redone here.

Research date 2026-09-20. Method: the Fluent syntax guide and spec (`projectfluent/fluent`), `fluent-rs` source at `main` (`fluent-bundle` 0.16.0), the Unicode MessageFormat working group repo (`unicode-org/message-format-wg`, editor's copy of the spec), ICU4X issues and pull requests, crates.io API metadata, each candidate crate's README and `Cargo.toml`, and the GNU gettext, ICU and Weblate documentation. Nothing was built or run; where a claim is an inference rather than read from a source, it says so. Download counts are cumulative from the crates.io API.

## 1. Answer

**Stay on Fluent.** No gap found is a blocker. Each real gap is either work the shared crate does anyway (formatting through ICU4X) or a gap every alternative shares or has worse (rich text, typed keys). The one alternative that is genuinely stronger on paper, Unicode MessageFormat 2 (MF2), has a stable spec but no usable Rust runtime today: ICU4X has not merged one and the only complete-looking Rust implementation is a six-week-old single-author 0.1.0 crate with 44 downloads.

What Ian probably noticed is real: `fluent-bundle` formats no numbers, dates or currency (section 3, gaps G1 and G2), has no inline-markup facility (G3) and no compile-time keys (G4). All four are addressable, and for G1 the earlier formatting note had already decided to bring in ICU4X regardless of the message system.

## 2. The project's needs

The fixed needs from the ticket, numbered so the tables can refer to them:

- N1: Locales `en-AU` (source), `en-US`, `en-GB` and a pseudo-Locale `en-XA`.
- N2: plurals as selectors.
- N3: no UI-framework dependency, usable from `bin-desktop` (gpui plus gpui-component with its own `rust-i18n`) and `bin-tui` (ratatui).
- N4: ICU4X for number, date and currency formatting.
- N5: inline styled or clickable spans inside one message.
- N6: hint messages that take key tokens as arguments.
- N7 (from the ticket's checklist): typed or compile-time-checked keys, gender and grammatical case for future Locales, plural categories for future Locales, message-level metadata and translator context, tooling, and upstream health.

## 3. Fluent's gaps, checked against spec and source

| Gap | Finding | Blocker? | Who fills it |
| --- | --- | --- | --- |
| G1 Number formatting | `NUMBER()` in Rust does not format: `builtins.rs` clones the number and merges options, and `FluentNumber::as_string` is `f64::to_string()` plus zero-padding for `minimumFractionDigits`. No grouping, no locale decimal separator, no currency. | No | Shared crate, via ICU4X (N4) |
| G2 Date formatting | `add_builtins` registers only `NUMBER`, with a literal `// TODO: DATETIME()` (`bundle.rs`); issue fluent-rs #181 is open. The third-party `fluent-datetime` 0.2.0 (2026-02-21) registers `DATETIME()` over `icu_datetime` 2. | No | Shared crate, via ICU4X, optionally `fluent-datetime` |
| G3 Rich text | Neither the syntax guide nor `fluent-bundle` has markup, formatted parts or spans: `format_pattern` and `write_pattern` return or write plain text. | Not a blocker, real cost | Shared crate (section 3.1) |
| G4 Typed keys | `fluent-bundle` looks up by string id. Ready-made answers exist: `i18n-embed-fl`'s `fl!` and `fluent-typed` 0.9.0. | No | Shared crate or a crate above |
| G5 Gender and case | Supported natively: terms take parameters, and selectors can match a term's attributes. | No | Fluent itself |
| G6 Plural categories | Supported, but `fluent-bundle` depends on `intl_pluralrules` 7.0.2, last published 2022-10-19 with `CLDR_VERSION = 37`. | No for English, watch for later Locales | See 3.2 |
| G7 Message metadata | Comments bound to a message (`#`) are shown to translators by tooling; group (`##`) and file (`###`) comments exist. | No | Fluent itself |
| G8 Tooling | Weblate supports Fluent but marks it under development; a Python linter exists; pseudo-localisation exists in `fluent-pseudo`. | No | See 3.3 |
| G9 Number precision | `FluentNumber` holds an `f64`, so `i64::MAX` is not representable (fluent-rs #337, open). | No | Shared crate passes money as a string or custom value |
| G10 Upstream health | Quiet, not dead. See 3.4. | Risk, not blocker | Escape hatches in section 6 |

Evidence for the rows above:

- G1: `fluent-rs/fluent-bundle/src/builtins.rs` and `types/number.rs` (`pub value: f64`, `as_string`). `FluentNumberOptions` parses `style` and `currency`, but `as_string` does not use them.
- G1 and the earlier formatting note: `FluentBundle::set_formatter` takes `Option<fn(&FluentValue, &M) -> Option<String>>` (`bundle.rs`), a plain function pointer, not a closure. The earlier formatting note describes it as a place for "a Locale-aware closure"; that is not what the signature allows, so a formatter needing state (an ICU4X formatter cache) cannot be captured there. The workable routes, all read from source, are `add_function`, which takes a `Fn + Sync + Send + 'static` closure but returns `FluentError::Overriding` if the id is already registered (so it cannot replace the built-in `NUMBER`), a custom `FluentType` whose `as_string` receives the bundle's `IntlLangMemoizer` (its doc comment cites a custom `DateTime`), and `fluent-datetime`. This is a correction to make in the formatting note, not a change of decision.
- G2: `fluent-bundle/src/bundle.rs` `add_builtins`; fluent-rs issues #181 (open since 2020, last updated 2025-09-18), #269 "Update FluentNumber to use ICU4X" and #329 "Using or switching to icu4x crates?" (both open); `fluent-datetime` `Cargo.toml` (`icu_datetime = 2`, `fluent-bundle = 0.16`).
- G3: `format_pattern` returns `Cow<str>` and `write_pattern` takes a `fmt::Write` (`bundle.rs`); `FluentValue` is `String`, `Number`, `Custom`, `None` or `Error` (`types/mod.rs`); a search of `fluent-bundle/src` for parts, overlay or markup found nothing.
- G5: `fluent/guide/terms.md` (parameterised terms with `$case` variants; term attributes carrying gender).
- G6: `intl_pluralrules` README ("cardinal plural rules ... from CLDR 35") and `rules.rs` (`CLDR_VERSION: usize = 37`); `fluent-bundle/Cargo.toml` depends on it. The type `FluentNumberType` has `Cardinal` and `Ordinal`, both handled in `FluentValue::matches`, and `intl_pluralrules` has an ordinal table (`PRS_ORDINAL`), so ordinals work. The ICU4X replacement `icu_plurals` is at 2.3.0 (2026-08-13, crates.io); fluent-rs has not moved to it.
- G7: `fluent/guide/comments.md`.
- G9: fluent-rs issue #337.

### 3.1 Rich text (N5, N6)

This is the gap with the most design cost, and it is a gap in the Rust runtime and the file format, not something a translator can work around inside a message. Fluent has no markup construct, so a message such as "Press `Enter` to save" where `Enter` is a styled span cannot say where the span starts and ends.

What works with Fluent (inference from the source read above; not built or tested here):

- Hint messages take the key token as a normal argument, `hint-save = Press { $key } to save`, and the caller substitutes a styled span for the placeholder. Because `format_pattern` returns one string, the shared crate has to recover the position of the placeholder: pass a sentinel string for each token (a private-use code point pair carrying an index), format the message, then split on the sentinels and wrap the tokens. `set_transform` (used by `fluent-pseudo`) is called only on textual fragments of the pattern (`bundle.rs` doc comment and `resolver/pattern.rs`), so the sentinels and the key tokens survive the `en-XA` transform untouched.
- N6 is therefore satisfied by this pattern. N5 in its stronger form, a styled span whose text is translated (for example a clickable word inside a sentence), needs either a message per span, or an agreed lightweight tag convention in the message text that the shared crate parses after formatting. The convention costs a small parser and a translator-facing rule; it is the same cost the alternatives below impose.

What MF2 offers: markup is part of the spec. The syntax has open, close and standalone markup (`{#link}...{/link}`), the data model resolves markup to a value with a type (open, standalone or close), an identifier and options, and resolution of markup "MUST always succeed" (`spec/syntax.md` and `spec/formatting.md`, "Markup Resolution"). This is the cleaner native answer to N5. It only pays off if a Rust runtime exposes the resolved parts, and none of the Rust MF2 code found does (section 4.1).

### 3.2 Plural rules for future Locales

For `en-AU`, `en-US`, `en-GB` and `en-XA`, CLDR 37 versus a current CLDR makes no difference: English has the categories `one` and `other`. The concern is only later Locales. `fluent-bundle` has a hard dependency on `intl_pluralrules`, so the plural data cannot be swapped without a fluent-rs change (the ICU4X migration issues above are open). A possible workaround, not tested: compute the category with `icu_plurals` in the shared crate and pass it as a string argument that the message selects on. This changes how translators write plural selectors and is best avoided unless a Locale actually needs newer rules. Any alternative built on ICU4X (MF2 through ICU4X, when it exists) would get current CLDR for free; this is the one point where the alternatives are cleanly ahead.

### 3.3 Tooling

- Weblate lists a Fluent format page that says "Support for this format is under development" and lists plural forms, context, explanation and location metadata as "No" in its capability table, with seven custom Fluent checks including syntax validation, parts, references and HTML tags (`docs.weblate.org/en/latest/formats/fluent.html`). The "plural forms: No" row describes Weblate's own plural feature; Fluent plurals are ordinary select expressions inside the message, so this is a limitation of the editing UI, not of the format.
- Mozilla's Pontoon reads Fluent and gettext through the `moz.l10n` message model (`pontoon/sync/formats/__init__.py` imports `fluent_astify_entry` and the `moz.l10n` `Format`, `Message`, `SelectMessage` types). Fluent is Mozilla's own format and is the best-supported case there.
- `moz-fluent-linter` (Python, MPL-2.0) lints Fluent for short or invalid identifiers and wrong characters and can restrict features such as attributes or variants.
- `fluent-pseudo` 0.3.3 provides the pseudo transform (see the binding note).
- Weblate has a "Pseudolocale generation" add-on (prefix and suffix; not format-aware). A search of Weblate's format documentation found no mention of MF2.

### 3.4 Upstream health

- Fluent syntax 1.0 shipped 2019-04-17 with "no breaking changes to the syntax and to the AST during the 1.x lifetime" (`fluent/spec/CHANGELOG.md`). The file format is stable.
- The `fluent-rs` workspace was released together on 2025-05-22 and 2025-05-23 by a new maintainer (alerque), after an ownership gap described by a Mozilla localisation lead in `projectfluent/fluent` #358 ("a mix of people leaving Mozilla or moving to different projects made the ownership of Fluent Rust unclear"). Since then `main` has had only lint, docs and typo commits (last on 2026-03-27); 14 pull requests are open (GitHub API, 2026-09-20). The syntax repo's last commits are lockfile and JS bundle updates on 2026-01-02.
- The same issue thread has the JS and Python implementation maintainer saying new development is slow "because it works", and that the Mozilla side considers Fluent still the target it is driving Firefox towards.
- `fluent-bundle` has 18.3M downloads and 5.8M recent, so it is not a fringe dependency. Slow-but-stable is the accurate reading; an unpatched bug in `fluent-bundle` is the exposure.
- Issue #349 in `projectfluent/fluent`, "Let's reduce the gap between Fluent and MessageFormat 2", is open (last updated 2022), and #358 is closed. I found no committed Fluent-to-MF2 migration path.

## 4. Alternatives

### 4.1 Unicode MessageFormat 2 (MF2)

The spec:

- The `message-format-wg` repo has a release tagged `LDML47-Stable` (2025-02-27); the latest is `LDML48.2` (2026-04-21). Its README says the Final Candidate specification was LDML 46.1, and that "During its development, Unicode MessageFormat was known as MessageFormat 2.0".
- The editor's copy has a stability policy: functions not marked Draft are Stable. In that copy `:number`, `:integer`, `:offset`, `:currency` and `:percent` are not marked Draft, while `:unit` and the whole of `datetime.md` (`:date`, `:time`, `:datetime`) are marked Draft. There is no list-formatting function in the default registry. The editor's copy may be ahead of the LDML 48.2 release, so "Stable" here is a reading of the current text, not of the release.
- MF2 defines a single message, not a file. The syntax spec lists "a future MessageResource specification" as a goal. So there are no message ids, no comments syntax and no file format: the project would have to choose or design a container (TOML, JSON, `.properties`), and translator context would live in that container. Attributes (`@name`) exist but "have no effect" on formatting and attach to expressions and markup, not to a whole message.
- Plural and select: `.match` on a `:number` or `:integer` variable with `select=plural` (the default) or `ordinal`; gender or case through `.match` on a `:string` value. Equivalent expressive power to Fluent selectors for our needs.

The Rust implementations:

| Item | Version, date | Maturity |
| --- | --- | --- |
| ICU4X | Issue #3028 "Implement MessageFormat 2.0" open; PR #7884 closed unmerged 2026-05-08 | Not implemented. `icu_experimental` 0.6.0 (2026-08-13) has `dimension`, `displaynames`, `duration`, `measure`, `personnames`, `relativetime`, `transliterate` and `units` modules, and no messageformat. A maintainer states there is an approved design and that someone else is working on it; the first pass "will have compiled APIs that link in all data" |
| `mf2_parser` / `mf2_printer` (lucacasonato) | 0.1.1 / 0.1.2, 2024-10-13 | Parser and printer only, 7.4k and 6.9k downloads. Also a language server, `mf2lsp`, "still in early development" |
| `ox_mf2_parser` | 0.14.0-alpha.12, 2026-08-10 | Parser core, alpha, 325 downloads |
| `mf2_i18n` (triesap) | 0.1.0, 2026-07-14 | Runtime, build tooling, std formatting backend and Leptos glue in one family of 0.1.0 crates; 44 downloads, 1 GitHub star, repository created 2026-06-21, one contributor; README: "This crate is pre-1.0" |

So there is no maintained Rust MF2 runtime with the ICU4X formatting the project wants. Choosing MF2 today means adopting a brand-new single-author crate, or writing the runtime (parser plus selection plus function registry plus ICU4X glue) in-house, plus designing the message container and building translator tooling. Weblate's format documentation has no MF2 entry that I could find.

### 4.2 ICU MessageFormat 1 (MF1)

- The ICU user guide describes `plural`, `select`, `selectordinal` and number, date and time arguments with styles or skeletons, and states that "A successor to this API is being developed in a working group ... MessageFormat 2.0". MF1 has no markup construct.
- No pure-Rust MF1 crate turned up in a crates.io search for "messageformat". The Rust route is `rust_icu_umsg` 5.8.0 (2026-08-20, 44k downloads, 279 recent), a binding to the ICU4C C API (`umsg.h`). `rust_icu`'s README says it is "not an officially supported Google product" and that features depend on the C API and a matching native ICU library. That conflicts with a self-contained AppImage, dmg and Windows zip and with the ICU4X plan.
- Legacy format, superseded by MF2 by the standards body's own account. Not viable.

### 4.3 gettext and PO

- `gettext-rs` 0.8.0 (2026-08-05, 4.7M downloads) and `gettext-sys`: by default `gettext-sys` "compiles and statically links its own bundled copy of GNU gettext", licensed LGPL; `gettext-system` uses the system library. Locale switching goes through the C library's process-wide state (inference from the C API, not tested here).
- `tr` 0.1.11 (2025-06-23) wraps gettext behind a `tr!` macro with `xtr` string extraction; its README lists gender or case, ICU-style formatting and localised date and number formatting as future plans. `gettext` 0.4.0 (pure-Rust `.mo` reader) was last published 2019-06-04. `i18n-embed` with the `gettext-system` feature composes `tr` and `gettext` (`i18n-embed/Cargo.toml`).
- Capabilities: the GNU manual's `ngettext (msgid1, msgid2, unsigned long n)` takes one number and a `Plural-Forms` expression per language; the manual's own multi-plural example, "one week and one day" versus "one week and %d days", shows the workaround for anything more. There is a `msgctxt` mechanism for disambiguation ("Using contexts for solving ambiguities"). There are no selectors for gender or case, no inline markup, no argument-typed formatting: every value is interpolated as text.
- Strength: the best translator tooling of any option (PO is supported by Weblate, Pontoon and most translation platforms).
- Verdict: fails N2 in spirit (plurals are function calls with one number, not selectors in the message), N5 and N6 (no markup, no formatted parts), and N7's grammatical features. A step backwards.

### 4.4 `rust-i18n` (and its relatives)

- `rust-i18n` 4.2.2 (2026-09-08, 3.5M downloads): YAML, JSON or TOML key-value files, compile-time embedding, fallback chains including `zh-CN` to `zh`, `%{name}` style variable interpolation and a `cargo i18n` extractor for untranslated text. The README has no plural, select, gender or formatting feature. That is why it is the wrong format for N2.
- Interaction with gpui-component: `gpui-component` 0.5.1 depends on `rust-i18n` 3 (`ui-stacks` note). This project's own dependency on rust-i18n 4 would be a second copy with its own `CURRENT_LOCALE` static. That is not a reason to pick it; it is a note that gpui-component's strings sit outside whichever system is chosen.
- `rosetta-i18n` 0.1.4 (JSON, "String formatting is supported", no plural or select in the README), `typed-i18n` 1.0.0 (YAML or JSON keyed per-language, `%{name}` interpolation, typed via a derive; nothing on plurals in the README) are typed-key generators for a flat string model. They give G4 but lose N2.

### 4.5 `i18n-embed`, `fluent-templates`, `fluent-zero`, `fluent-typed`, `l10n`

These are Fluent bindings, evaluated in the binding note; they change how Fluent is loaded, not the message system. Additional facts found here: `fluent-typed` 0.9.0 (2026-08-14, 18k downloads, MIT) generates one typed accessor per message and checks every locale for the same variables, with unused-message warnings and a documented comparison to `fl!`; it is young. `l10n` 0.1.3 offers compile-time checks over `fluent-bundle` but has 21 recent downloads. `fluent-zero` 0.1.4 (2026-03-28) generates static lookups for allocation-free GUI use.

### 4.6 `leptos_i18n`, Slint and other framework-tied crates

`leptos_i18n` 0.6.2 (2026-04-14) has a non-optional dependency on `leptos` in its `Cargo.toml`, so it breaks N3 (and the project has no Leptos crate). It is designed for Leptos signals and views. Slint's built-in gettext translation, `dioxus-i18n` 0.5.1 and `egui_i18n` 0.2.0 are likewise tied to their UI frameworks. None can serve both `bin-desktop` (gpui) and `bin-tui` (ratatui). I read only the manifests and READMEs; I did not evaluate their message formats in depth.

## 5. Comparison against the needs

| Need | Fluent | MF2 | MF1 | gettext | rust-i18n |
| --- | --- | --- | --- | --- | --- |
| N1 Locales, pseudo | Yes, `fluent-pseudo` transform on text only | Format allows it; no Rust pseudo tool found | Not applicable | Weblate add-on only | Not native |
| N2 Plural selectors | Yes (CLDR 37 data) | Yes, `:number` and `:integer` | Yes | Function-level, one number | No |
| N3 No UI framework | Yes | Yes | No, needs ICU4C | Bundled C library or old pure-Rust reader | Yes |
| N4 ICU4X formatting | Yes, via custom function, `FluentType` or `fluent-datetime` | Would need a runtime first | No | No | No |
| N5 Inline spans | No, sentinel or tag convention | Yes in the spec, no Rust runtime | No | No | No |
| N6 Key-token arguments | Yes | Yes in the spec | Yes | Text interpolation | Text interpolation |
| Typed keys | `fluent-typed`, `fl!` | None | None | None | `t!` string keys |
| Gender and case | Yes | Yes | `select` | No | No |
| Message metadata | Comments in the format | None (container-defined) | None | Yes, PO comments and context | None |
| Rust maturity | Stable, quiet | Not usable | Wrong dependency | Mature, C dependency | Mature, wrong model |

## 6. Blockers versus work the shared crate does anyway

Not blockers, and to be built in `lib-localisation` whichever message system wins:

- Number, date and currency formatting through ICU4X (G1, G2, N4): the same work for MF2, because no MF2 runtime found wraps ICU4X for us. A message system only decides how a message reaches the formatter.
- Typed or checked keys (G4): `fluent-typed`, `fl!` or a small `build.rs` over `fluent-syntax`. No alternative gives this for free.
- Placeholders and spans (G3, N5, N6): a sentinel or tag convention. MF2's markup is the only native answer, but it needs a Rust runtime that exposes parts, which nobody has shipped.
- Money as a string or custom value (G9), not `f64`.

Real costs that remain on Fluent:

- G3 is a genuine expressiveness gap, not merely missing tooling; it is a convention the project owns.
- G6 and G10 are exposure to a slow upstream. Mitigation: the `.ftl` format is stable and independent of the Rust crate, `fluent-syntax` parses it, and the shared crate's lookup API is the only place a replacement runtime would be plugged in. Keeping message ids and the lookup API independent of Fluent types is what keeps a later switch cheap.

## 7. Recommendation and consequences for the map

Stay on Fluent, with `fluent-bundle` or the binding chosen in the earlier note.

- The ICU4X decision stands. Correct the formatting note's `set_formatter` wording: it takes a function pointer, so wire ICU4X through `add_function` (under a new name, since `NUMBER` is already registered), a custom `FluentType`, or `fluent-datetime` for dates.
- Add to the map's open decisions: how inline spans are expressed (sentinel placeholders for key tokens, and a tag convention only if translated styled spans are needed). This is the one place a design decision, not just code, follows from this research.
- No change to the Locale set or the `en-XA` approach.
- Revisit the choice if any of these happen: ICU4X ships MF2 under `icu` (track issue #3028); fluent-rs stops releasing or merging (currently 14 open pull requests, last release May 2025); or the project needs a Locale whose plural rules differ between CLDR 37 and current and fluent-rs still has not adopted `icu_plurals`.
- To keep the door open, do not let `fluent-bundle` types leak through `lib-localisation`'s public API: expose ids, arguments and a result that is text plus optional spans.

## 8. Not verified

- No code was built or run. The sentinel-and-split approach, the `icu_plurals` workaround and `fluent-datetime` working with this project's ICU4X versions are inferences from source and manifests.
- Whether `moz.l10n` or any tool converts Fluent to MF2 was not established; only that Pontoon's source uses a shared message model for Fluent and gettext.
- The exact content of MF2's stable and Draft function lists in the LDML 48.2 release, as opposed to the editor's copy, was not compared.
- Weblate's MF2 support: the format documentation excerpt I fetched did not mention it, which is weaker than confirming absence. Other translation platforms (Crowdin, Lokalise and similar) were not checked for Fluent or MF2 support.
- The behaviour of `gettext-rs` around process-wide locale state, and the MF1 (ICU) specification text beyond the user guide, were not checked in source.
- Behaviour of `leptos_i18n`, `rosetta-i18n` and `typed-i18n` formats was read from README and manifest only.
- Binary size and build time for any option were not measured.

## Sources

- Fluent syntax guide, spec and changelog: `https://github.com/projectfluent/fluent` (`guide/terms.md`, `guide/comments.md`, `spec/CHANGELOG.md`); issues #358 and #349.
- `fluent-rs` source: `https://github.com/projectfluent/fluent-rs` (`fluent-bundle/src/bundle.rs`, `builtins.rs`, `types/mod.rs`, `types/number.rs`, `resolver/pattern.rs`); issues #181, #269, #329 and #337; commit and pull request history via the GitHub API.
- `intl_pluralrules`: `https://github.com/zbraniecki/pluralrules` (`README.md`, `intl_pluralrules/src/lib.rs`, `rules.rs`).
- `fluent-datetime`: `https://github.com/g2p/fluent-datetime` (`README.md`, `Cargo.toml`).
- `fluent-typed`, `fluent-zero`, `l10n`, `i18n-embed`, `fluent-templates`: their repositories' `README.md` and `Cargo.toml` (`https://github.com/human-solutions/fluent-typed`, `https://github.com/xangelix/fluent-zero`, `https://github.com/MathieuTricoire/l10n`, `https://github.com/kellpossible/cargo-i18n`, `https://github.com/XAMPPRocky/fluent-templates`).
- Unicode MessageFormat: `https://github.com/unicode-org/message-format-wg` (`spec/README.md`, `spec/intro.md` stability policy, `spec/syntax.md`, `spec/formatting.md`, `spec/functions/*.md`, releases list).
- ICU4X: `https://github.com/unicode-org/icu4x` issues #3028 and #6134, pull request #7884, and the `components/experimental/src` directory listing.
- Rust MF2 crates: `https://github.com/lucacasonato/mf2-tools`, `https://github.com/triesap/mf2_i18n`, and crates.io entries for `mf2_parser`, `mf2_printer`, `ox_mf2_parser`, `mf2_i18n`.
- ICU MessageFormat 1: `https://unicode-org.github.io/icu/userguide/format_parse/messages/`; `rust_icu`: `https://github.com/google/rust_icu` (`README.md`).
- gettext: `https://www.gnu.org/software/gettext/manual/html_node/Plural-forms.html`; `https://github.com/gettext-rs/gettext-rs` (`gettext-sys/README.md`); `https://github.com/woboq/tr` (`README.md`).
- `rust-i18n`, `rosetta-i18n`, `typed-i18n`, `leptos_i18n`: `https://github.com/longbridge/rust-i18n`, `https://github.com/baptiste0928/rosetta`, `https://github.com/alexkazik/typed-i18n`, `https://github.com/Baptistemontan/leptos_i18n` (`README.md`, `Cargo.toml`).
- Weblate: `https://docs.weblate.org/en/latest/formats/fluent.html`, `https://docs.weblate.org/en/latest/formats.html`, `https://docs.weblate.org/en/latest/admin/addons.html`. Pontoon: `https://github.com/mozilla/pontoon` (`pontoon/sync/formats/__init__.py`). Fluent linter: `https://github.com/mozilla-l10n/moz-fluent-linter`.
- crates.io API metadata (`https://crates.io/api/v1/crates/<name>`) for every crate named, queried 2026-09-20.
- Earlier notes on this map: `docs/research/localisation-fluent-binding.md`, `localisation-formatting.md` and `localisation-ui-stacks.md` (branches `origin/research/localisation-fluent-binding`, `-formatting`, `-ui-stacks`).
