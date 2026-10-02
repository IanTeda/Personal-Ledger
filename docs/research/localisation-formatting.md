# Research: Locale-aware number, date and currency formatting for `lib-localisation`

**Question.** [Issue #213](https://github.com/IanTeda/Personal-Ledger/issues/213), a child of the Localisation map ([#211](https://github.com/IanTeda/Personal-Ledger/issues/211)): `lib-localisation` must render numbers, dates and currency amounts per Locale for `en-AU`, `en-US` and `en-GB`. This note compares ICU4X, Fluent's built-in `NUMBER()`/`DATETIME()`, `chrono`'s locale support, and hand-rolled tables driven by the existing `lib-core` `NumberFormat` and `DateFormat` types, then recommends one. It covers separators, date orderings and month/weekday names, currency symbol placement, how a non-currency Unit (a ticker such as AAPL, bitcoin) is formatted, precision when amounts are held as `bigdecimal`, binary-size and compile cost, and cooperation with a Fluent binding.

Research was done by reading the ICU4X source and READMEs (`unicode-org/icu4x`, `components/decimal`, `components/datetime`, `components/experimental/src/dimension/currency`, `utils/fixed_decimal`, `tutorials/data-management.md`), the `fluent-rs` source (`fluent-bundle/src/builtins.rs`, `types/number.rs`, `types/mod.rs`, `bundle.rs`), the Project Fluent guide (`projectfluent.org/fluent/guide/functions.html`), `chrono`'s `Cargo.toml`, README and `src/format/locales.rs`, crates.io metadata, and this repo's `lib-core` (`number_format.rs`, `date_format.rs`, `money.rs`, `unit_kind.rs`) and `docs/research/chrono-alternatives.md`. Claims marked "measured" come from a throwaway probe crate (not committed) built against the crate versions below with `opt-level = "s"`, LTO and `strip` on this machine (Linux x86_64).

## 1. What the workspace has today

- `NumberFormat` (`lib-core`, ADR-0014) is a three-variant enum coupling thousands and decimal separators: `1,234.56`, `1.234,56` and `1 234,56`. It exposes `thousands_separator()` and `decimal_separator()` and nothing else: no grouping size, no currency placement, no negative-number style.
- `DateFormat` (`lib-core`) is a three-variant enum (`DayMonthYear` `31/12/2026`, `MonthDayYear` `12/31/2026`, `Iso` `2026-12-31`) that maps to a `chrono` strftime pattern via `pattern()`. It has no month or weekday names.
- `Money` (`lib-core`) wraps `bigdecimal::BigDecimal` and is stored in SQLite as exact text, so amounts are never floats.
- `UnitKind` (`lib-core`) groups a Unit as Fiat, Crypto, Stock, PreciousMetal or Other. It carries no symbol or placement data.
- `chrono` is retained as the workspace date library (`docs/research/chrono-alternatives.md` found no deprecation and a first-party `sqlx` integration).

## 2. ICU4X

**Versions (crates.io, queried directly):** `icu` 2.3.1, `icu_decimal` 2.3.0, `icu_datetime` 2.3.0, `icu_provider` 2.3.1, `fixed_decimal` 0.7.2, `icu_experimental` 0.6.0.

**Numbers.** `icu_decimal::DecimalFormatter` formats a `fixed_decimal::Decimal` per Locale, including grouping and numbering systems (its README shows Bangla digits and `2,000.50`). Measured for `-1234567.8900`: `en-AU`, `en-US` and `en-GB` all give `-1,234,567.8900` (trailing zeros are preserved because `Decimal` carries its own scale). The decimal README states that currency, measurement units and compact notation are "planned" in `icu_decimal` itself.

**Dates.** `icu_datetime::DateTimeFormatter` uses UTS 35 semantic skeletons (`fieldsets::YMD::short()`, `medium()`, `long()`, `fieldsets::YMDE`) and formats `icu_calendar::Date`, `DateTime` and `ZonedDateTime`. Measured for 3 September 2026:

| Locale | `YMD::short` | `YMD::medium` | `YMDE::long` |
| --- | --- | --- | --- |
| `en-AU` | `3/9/26` | `3 Sept 2026` | `Thursday 3 September 2026` |
| `en-US` | `9/3/26` | `Sep 3, 2026` | `Thursday, September 3, 2026` |
| `en-GB` | `03/09/2026` | `3 Sept 2026` | `Thursday, 3 September 2026` |

Two things to note. `en-AU` short uses a two-digit year and no zero padding, so it does not reproduce `DateFormat::DayMonthYear`'s `31/12/2026`; and the CLDR `en-AU`/`en-GB` abbreviation of September is `Sept`. Both are what the Locale's own convention says, which is the point of moving to Locale-owned formatting, but the reconciliation ticket should decide whether the ISO `2026-12-31` option survives as an explicit override (no ICU4X field set was found that means "always ISO", so it would be a hand-formatted special case).

**Chrono interop.** `icu_datetime`'s `lib.rs` has `#[cfg(feature = "unstable_chrono_0_4")] mod chrono;` and its README lists "datetime types from third party crates; see `input::unstable_third_party`". So a `chrono::NaiveDate` can be fed in behind an explicitly unstable feature; the stable path is `Date::try_new_iso(y, m, d)` from `chrono`'s `Datelike` accessors, which costs three lines and no unstable feature.

**Currency.** Lives in `icu_experimental` (0.6.0, semver-unstable, `dimension::currency::formatter::CurrencyFormatter`), not in `icu_decimal`. Constructors include `try_new_symbol`, `try_new_symbol_narrow`, `try_new_code`, `try_new_name` and `try_new_no_currency`, with `CurrencyUsage::Accounting` for parenthesised negatives. Measured for `-1234567.89`:

| Locale | AUD | USD | GBP | EUR | JPY | BTC |
| --- | --- | --- | --- | --- | --- | --- |
| `en-AU` | `-$1,234,567.89` | `-US$1,234,567.89` | `-£1,234,567.89` | `-€1,234,567.89` | `-JP¥1,234,568` | `-BTC 1,234,567.89` |
| `en-US` | `-A$1,234,567.89` | `-$1,234,567.89` | `-£1,234,567.89` | `-€1,234,567.89` | `-¥1,234,568` | `-BTC 1,234,567.89` |
| `en-GB` | `-A$1,234,567.89` | `-US$1,234,567.89` | `-£1,234,567.89` | `-€1,234,567.89` | `-JP¥1,234,568` | `-BTC 1,234,567.89` |

The symbol for the home currency is context-sensitive (`$` for AUD in `en-AU`, `A$` elsewhere), the currency's own minor-unit count is applied (JPY rounds to 0 decimals), and an ISO code with no CLDR symbol (BTC) falls back to the code with a space. Accounting usage gave `($1,234,567.89)` for negatives.

**Non-currency Units.** ICU4X has no formatter for tickers. `CurrencyFormatter::try_new_no_currency` formats just the number with the currency's fraction digits (the source doc example is `12,345.67`), and the caller appends or prepends the ticker. Because a ticker such as AAPL has no CLDR entry, the sensible approach is to format the quantity with `DecimalFormatter` (choosing the fraction digits from the Unit, not a currency) and place the code as a suffix, `1,234.5 AAPL`, or hand a Fluent message the two pieces so translators own the placement.

**Precision.** `fixed_decimal::Decimal` is arbitrary-precision decimal, not a float. Measured: `0.1234567890123456789012345678` and `123456789012345678901234567890.123456789` round-trip exactly through `Decimal::from_str` and formatting. There is no `bigdecimal` bridge in `fixed_decimal` (its only optional dependency is `ryu`), so the bridge is a string. Gotcha, measured: `BigDecimal::to_string()` (the `Display` that `Money` uses) emits scientific notation for extreme scales (`1e+30`, `1E-26`), and `Decimal::from_str("1e+30")` fails with a syntax error (the `E-26` form parsed). `BigDecimal::to_plain_string()` gave `1000000000000000000000000000000` and parsed cleanly. So the bridge must use `to_plain_string()`, never `Display`. Rounding to a Unit's displayed scale should be done on the `Decimal` (`rounded`, `round_with_mode`) at the display boundary only; the stored `Money` is untouched.

**Binary size and compile cost (measured).** A probe with `icu_decimal` + `icu_datetime` (compiled data, all locales, the three formatters above) linked to a 1.47 MB stripped release binary; a probe with `icu_experimental` currency (which pulls `icu_decimal` and `icu_plurals`) was 0.39 MB without datetime. Expect roughly 1.5 to 2 MB of ICU4X data and code combined with default compiled data, including every CLDR locale. A cold release build of the datetime+decimal probe took about 22 seconds wall clock (about 40 seconds CPU) on this machine, with ICU4X's ~15 small crates as the cost. Size can be cut by generating data for only `en-AU`, `en-US` and `en-GB` (and `en-XA` if wanted) with `icu4x-datagen` and pointing `ICU4X_DATA_DIR` at it at compile time (`tutorials/data-management.md`: "It should result in a smaller binary, as we're including only a single locale"), or by static field sets (the `icu_datetime` README's "Binary Size Considerations" says static field sets let the compiler drop unneeded data, whereas dynamic ones link every pattern). The generated data can be checked in or built in CI. This was not measured here because it needs `cargo install icu4x-datagen`.

**Fluent cooperation.** ICU4X does not depend on Fluent and vice versa. See section 3 for the hook.

## 3. Fluent's built-in `NUMBER()` and `DATETIME()`

- The Fluent guide defines `NUMBER` (mirroring `Intl.NumberFormat` options) and `DATETIME` (mirroring `Intl.DateTimeFormat`) as built-ins that "FTL implementations should ship".
- In `fluent-rs` (`fluent-bundle` 0.16.0, crates.io last published 2025-05-22), `builtins.rs` implements only `NUMBER`, and it does no formatting: it clones the `FluentNumber` and merges the named arguments into `FluentNumberOptions`. `bundle.rs` at `add_builtins` contains a literal `// TODO: DATETIME()`, so `DATETIME()` does not exist in the Rust implementation.
- `FluentNumber` stores `value: f64` (the `bigdecimal` amount would be squeezed through a float) and its `as_string()` is `self.value.to_string()` plus zero-padding for `minimumFractionDigits`. There is no grouping, no locale-specific decimal separator, and no currency formatting: `FluentNumberOptions` parses `style`, `currency` and `currencyDisplay` but the `as_string()` shown in `number.rs` does not use them beyond `minimumFractionDigits`. The only Locale-aware piece of number handling in `fluent-bundle` is plural category selection (via `intl_pluralrules`), which is what selectors such as `[one]`/`[other]` use.
- Extension points that make ICU4X (or anything) pluggable: `FluentBundle::set_formatter` (called first in `FluentValue::write`, so a Locale-aware closure can render every `FluentValue`), `add_function` for custom functions, and the `FluentType` trait for custom values (its doc comment cites "a custom `DateTime`"), whose `as_string(&self, intls)` is handed the bundle's memoiser.

Verdict: Fluent's `NUMBER()`/`DATETIME()` are not a formatter in Rust and are unsuitable as the formatting engine. They are the right syntax surface, though: a message can say `{ NUMBER($amount, minimumFractionDigits: 2) }` and be resolved by an app-supplied `set_formatter` or custom function that calls the real formatter.

## 4. `chrono` locale support

- `chrono`'s `unstable-locales` feature (`Cargo.toml`: `unstable-locales = ["pure-rust-locales"]`; README: "Enable localization. This adds various methods with a `_localized` suffix") gives `format_localized`. Its `Locale` enum matches "the locales in glibc" and `src/format/locales.rs` reads only `LC_TIME` names/patterns and `LC_NUMERIC::DECIMAL_POINT` (`pure-rust-locales` 0.8.2: "imported directly from the GNU C Library"). The feature is explicitly "unstable".
- Measured for 3 September 2026: `%x` gives `03/09/26` (`en_AU`), `09/03/2026` (`en_US`), `03/09/26` (`en_GB`); month and weekday names are the same English names in all three; abbreviations (`%b`) are `Sep` in all three. So it captures ordering but not the CLDR differences ICU4X gives (`Sept`, `Thursday, 3 September` comma placement).
- No number or currency formatting: only the decimal point is exposed, not grouping, and nothing for currency. Decimal amounts do not pass through `chrono` at all.
- Benefit: no new date dependency (chrono is already there), tiny cost. Limit: dates only, glibc-derived rather than CLDR, unstable feature flag.

## 5. Hand-rolled tables on `NumberFormat` and `DateFormat`

- Feasible for exactly the three Locales in scope, since `en-AU`, `en-US` and `en-GB` all use `,` grouping and `.` decimal and differ only in date order, a handful of month abbreviations, and currency symbol placement. `NumberFormat` already supplies separators; `DateFormat` already supplies strftime patterns.
- Gaps to fill by hand: grouping size assumptions (all three use groups of three, fine), negative-number style (`-$1.00` vs `($1.00)`), currency symbols per Locale-and-currency pair (the `A$`/`US$` disambiguation shown above is exactly the sort of rule that rots), minor-unit counts per currency (JPY 0, BHD 3), and a placement rule for tickers, plus every new Locale needing new tables.
- Cost: zero dependencies and binary size, full control, trivially Fluent-friendly (a plain function). Risk: it re-implements CLDR badly, and `NumberFormat`'s own three-variant design (which includes `1 234,56`) already exceeds what the three in-scope Locales need while still lacking anything for currency. The map's destination reconciles these Preferences with the Locale; hand-rolling keeps them as a parallel system in miniature.

## 6. Comparison

| Concern | ICU4X | Fluent built-ins (Rust) | `chrono` locales | Hand-rolled |
| --- | --- | --- | --- | --- |
| Number separators/grouping | Yes, CLDR | No (f64 `to_string`) | Decimal point only | Yes (existing enum) |
| Date order, month/weekday names | Yes, CLDR | No (`DATETIME` is a TODO) | Yes, glibc-derived | Only order, by hand |
| Currency placement and symbol | Yes, unstable (`icu_experimental`) | No | No | By hand |
| Non-currency Unit (ticker) | No formatter; number via `DecimalFormatter` or `no_currency`, ticker by caller | No | No | By hand |
| Exact decimals | Yes via `Decimal` (string bridge, use `to_plain_string`) | No (f64) | N/A | Yes (string ops on `BigDecimal`) |
| Size/compile | About 1.5 to 2 MB default, cut by datagen; about 15 crates | Already present | Tiny, one optional crate | None |
| Fluent cooperation | Via `set_formatter`/custom functions | Native syntax only | Same as hand-rolled | Same |

## 7. Recommendation

**Use ICU4X for formatting, behind a thin `lib-localisation` wrapper, with Fluent providing only message syntax and calling into it.** Specifically:

- `icu_decimal` + `icu_datetime` for numbers and dates (both are stable 2.x crates), `icu_experimental`'s `CurrencyFormatter` for fiat currency amounts, wrapped so the unstable API is one file and can be swapped.
- Units that are not fiat currencies (tickers, bitcoin as a `UnitKind::Crypto`/`Stock`) are formatted as a number through `DecimalFormatter` with Unit-specific fraction digits plus the Unit code, with the code placement (`{ $quantity } { $code }`) living in a Fluent Message so translators own it. Fiat Units go through `CurrencyFormatter`; codes with no CLDR symbol (BTC) already degrade to the code plus space.
- Amounts stay `Money(BigDecimal)`; the wrapper converts with `to_plain_string()` into `fixed_decimal::Decimal` and rounds there. Never `Display`, never `f64`. Register a Fluent function or `set_formatter` so `{ $amount }` and `NUMBER()` route through the wrapper instead of `FluentNumber::as_string`.
- Ship compiled data for only the four Locales with `icu4x-datagen` and `ICU4X_DATA_DIR` if the default 1.5 to 2 MB matters for the TUI's AppImage/dmg/zip; otherwise defaults are acceptable for a desktop-class app.
- Keep `chrono` for storage and pass dates in via `Date::try_new_iso`; do not enable `unstable-locales` (dates only, glibc-derived, unstable, redundant once ICU4X is in).

**Tradeoffs.** Pros: correct CLDR output for all three Locales (including the `A$`/`US$` disambiguation and `Sept` abbreviation) with no hand-maintained tables, exact decimals, and a route to more Locales for free. Cons: a real dependency and compile-time cost (about 15 crates, 22 seconds cold in the probe), `icu_experimental` currency is semver-unstable at 0.6.0 (pin and wrap), no ticker support (so a small hand-written rule is still needed), and the CLDR short `en-AU` date (`3/9/26`) differs from the current `DateFormat::DayMonthYear` (`31/12/2026`), so the Preference reconciliation ticket must decide what the "date format" Preference becomes (a length choice `short`/`medium`/`long` plus an explicit ISO override is the natural mapping).

**Fallback if the dependency is unwelcome:** hand-rolled tables for `en-AU`, `en-US` and `en-GB` only (small, no new crates), keeping `DateFormat`/`NumberFormat` as the source and adding a currency-placement table. Acceptable for the current three Locales but it does not scale and must reimplement currency rules.

**Not recommended:** Fluent built-ins alone (they do not format in Rust and lose precision through `f64`), or `chrono` locales as the date engine.

## 8. Sketch

```rust
// lib-localisation/src/format.rs (illustrative, not built)
use fixed_decimal::Decimal;
use icu_datetime::{DateTimeFormatter, fieldsets, input::Date};
use icu_decimal::DecimalFormatter;
use icu_experimental::dimension::currency::formatter::CurrencyFormatter;
use icu_locale::Locale;

pub struct Formatters {
    number: DecimalFormatter,
    date_short: DateTimeFormatter<fieldsets::YMD>,
    locale: Locale,
}

impl Formatters {
    /// Fiat amounts: `Money` -> exact `Decimal` -> Locale currency format.
    pub fn currency(&self, amount: &Money, iso: CurrencyType) -> Result<String, FormatError> {
        let dec: Decimal = amount.0.to_plain_string().parse()?; // never Display: `1e+30` fails
        let fmt = CurrencyFormatter::try_new_symbol(self.locale.clone().into(), iso, Default::default())?;
        Ok(fmt.format_fixed_decimal(&dec).write_to_string().into_owned())
    }

    /// Non-fiat Units: number via DecimalFormatter; code placement is a Fluent Message.
    pub fn quantity(&self, amount: &Money) -> Result<String, FormatError> {
        let dec: Decimal = amount.0.to_plain_string().parse()?;
        Ok(self.number.format(&dec).write_to_string().into_owned())
    }
}
// Fluent: bundle.set_formatter(Some(...)) or add_function("MONEY", ...) delegates to Formatters.
```

## 9. Open points for the wider map

- Whether `icu_experimental`'s currency API is stable enough at 0.6.0 for a synced build; the wrapper isolates it, but a follow-up should re-check on each ICU4X minor release.
- The Preference migration (map "Not yet specified"): `NumberFormat`'s `SpaceThousandsCommaDecimal` and `DateFormat::Iso` have no obvious home once the Locale owns formatting; this note only shows that `en-AU`/`en-US`/`en-GB` need neither.
- The `en-XA` pseudo-Locale has no CLDR data in ICU4X; formatting under `en-XA` should fall back to `en-AU` formatters (a fallback decision for the negotiation ticket, not this one).
- The datagen size cut was not measured and should be confirmed by the build ticket before committing to it.
