# Research: Parsing Locale-shaped date input and relative words for `lib-localisation`

**Question.** [Issue #225](https://github.com/IanTeda/Personal-Ledger/issues/225), a child of the Localisation map ([#211](https://github.com/IanTeda/Personal-Ledger/issues/211)): typed date input follows the Locale's short form by default, always also accepts ISO (`YYYY-MM-DD`), and accepts the relative words `today`, `yesterday` and `tomorrow` (ADR-0021). ICU4X formats dates but does not parse them. This note answers four questions: (1) can the parser be derived from the Locale's resolved short pattern rather than hand-tabled, (2) does an existing crate parse Locale-ordered numeric dates and relative words for `en-AU`, `en-US` and `en-GB`, (3) how the accepted relative words can come from a Message per Locale with English always accepted, and (4) how year-less and two-digit-year input and error reporting should work. It builds on the ICU4X findings in `docs/research/localisation-formatting.md` (branch `origin/research/localisation-formatting`).

Research was done by reading the published crate sources from static.crates.io (`icu_datetime` 2.3.0, `chrono` 0.4.45, `chrono-english` 0.2.1, `interim` 0.2.1, `dateparser` 0.3.1), crates.io metadata, and this repo's `crates/bins/bin-desktop/src/transaction_filter_form.rs` (`parse_date`, `parse_iso`, `parse_slash`, `parse_day_month_year`). Claims marked "measured" come from a throwaway probe crate (not committed) built against `icu` 2.3.1 (with the `unstable` feature enabled) and `chrono-english` 0.2.1 on this machine.

## 1. What the repo does today

- `parse_date(text, today, date_format)` in `bin-desktop` trims, treats an empty string as "no bound", lower-cases, accepts the single word `today`, always tries ISO, and then tries one of two hand-written parsers chosen by the `DateFormat` Preference: `parse_day_month_year` (`01 jan 2026`, matched against an English `MONTH_ABBREVIATIONS` table) or `parse_slash` (`d/m/yyyy`). `DateFormat::Iso` adds nothing beyond the always-on ISO attempt.
- Both hand parsers require a four-digit year (`year.len() != 4`), reject year-less input, and hard-code day-before-month. `yesterday` and `tomorrow` do not exist.
- Errors are a `String` built at the call site (`use 01 jan 2026, 2026-01-01 or today`), with the example produced by formatting today's date in the chosen style. That pattern (an example rendered through the formatter) is worth keeping; the English literals are not.
- The TUI has no equivalent typed date parser in the files searched (its `NaiveDate` uses are fixtures and views).

## 2. Does ICU4X expose the resolved short pattern?

Yes, as a string, and as a structured component bag. Source: `icu_datetime` 2.3.0 `src/neo.rs` and `src/pattern/pattern.rs`.

- `FormattedDateTime::pattern()` ("Gets the pattern used in this formatted value") returns a `DateTimePattern`, which implements `Writeable`/`Display`, so it renders as the UTS 35 pattern string. This is the pattern actually used for the given date, from `DateTimeFormatter::try_new(locale, fieldsets::YMD::short())` then `.format(&date).pattern()`.
- `components::Bag::from(&pattern)` (module `icu_datetime::provider::fields::components`; the `fields` module is not feature-gated in `src/provider/mod.rs`, though the probe enabled `unstable` and the gating of the exact path was not re-verified without it) turns the pattern into a struct with `year`, `month`, `day` and their widths (`Numeric`, `TwoDigit`, `NumericDayOfMonth`, `TwoDigitDayOfMonth`). It does not carry field order or the literal separator, so it answers "two-digit year? padded day?" but not "day first?".
- `DateTimePattern::iter_items` (which would give order and literals structurally) is `pub(crate)`. The public route to order and separator is therefore the pattern string.

Measured for 3 September 2026 with `fieldsets::YMD::short()`:

| Locale | Resolved short pattern | Bag year / month / day |
| --- | --- | --- |
| `en-AU` | `d/M/yy` | TwoDigit / Numeric / NumericDayOfMonth |
| `en-US` | `M/d/yy` | TwoDigit / Numeric / NumericDayOfMonth |
| `en-GB` | `dd/MM/y` | Numeric / TwoDigit / TwoDigitDayOfMonth |

`YMD::medium()` gave `d MMM y` (`en-AU`, `en-GB`) and `MMM d, y` (`en-US`), which also shows the month-name form the current `parse_day_month_year` handles.

So the parser can be derived: scan the pattern string, take the order of the first occurrences of `d`, `M` and `y` (any run of that letter), and take the first non-letter literal as the separator (`/` for all three here). The pattern letters are UTS 35 field symbols (`d` day of month, `M` month, `y` year), so this is a stable reading of a documented syntax, not a guess at ICU internals. The parser then reads digits for each field regardless of the padding the pattern shows: the pattern says how to print (`dd`, `yy`), not how strictly to read, and typed input should be lenient on padding.

What a derived parser cannot cover, and needs a fallback for:

- Locales whose short pattern has a non-numeric field (medium/long patterns, or month names). Restrict typed input to the numeric short pattern plus ISO and let month-name input be a separate, optional extension, since month names would need `icu_datetime`'s name data reversed, which ICU4X does not offer.
- Non-Gregorian calendars or non-Latin digits: out of scope for `en-AU`/`en-US`/`en-GB`, but derived parsing should read only ASCII digits and fail clearly otherwise.
- The current pattern is `d/M/yy`: formatting `en-AU` output and then parsing it back needs two-digit years accepted (section 5).

Cost: ICU4X is already the recommended formatter, so this adds no dependency. A cheap alternative for the three in-scope Locales is a three-row table of (order, separator), but deriving from the pattern avoids the table rotting when a Locale is added and guarantees the parser accepts exactly what the formatter prints.

## 3. Existing crates

- `chrono` (0.4.45): `NaiveDate::parse_from_str` / `parse_and_remainder` take a strftime format, so a derived order becomes a format string. It is the storage type already, and `%y` follows a fixed pivot (source `src/format/strftime.rs` note: values greater or equal to 70 are the 20th century, others the 21st). It has no relative words and no notion of a Locale.
- `chrono-english` 0.2.1 (crates.io: last published 2026-08-26, about 4.1M downloads) and its fork `interim` 0.2.1 (2025-03-05, about 1.4M): English only, with `Dialect::Uk` (day first) or `Dialect::Us` (month first). Measured for a base of 20 September 2026: `today`, `yesterday`, `tomorrow` resolve correctly; `03/09/2026` is 3 September under `Uk` and 9 March under `Us`; `13/09/2026` is valid only under `Uk` and `09/13/2026` only under `Us` (both otherwise return "bad date"); ISO and `3 Sep 2026` parse under both; year-less `3/9` takes the current year; two-digit years use a fixed 1940 to 2040 window (`1/2/39` is 2039, `1/2/70` is 1970). It also accepts `next friday 8pm`, `6 months ago` and so on, far more than the spec asks for, and it needs a base `DateTime`. Its errors are plain English strings (`"bad date"`, `"expected week day or month name"`) via `DateError`, so they cannot become Messages. The `Dialect` is a two-value enum, so `en-AU` maps to `Uk` by convention; `hoy` fails, so there is no Locale-word extensibility.
- `dateparser` 0.3.1 (2026-03-25, about 4.9M downloads): its lib.rs named formats are `slash_mdy`, `month_mdy`, `month_dmy`, `dot_mdy_or_ymd` and so on, i.e. month-first for numeric slash dates, with no day-first option found and no relative words (its scope is "commonly used" absolute formats and timestamps). It would misread `en-AU` `3/9/26`.
- `two-timer` 2.2.5 (2023-10-10, about 53k downloads): English time-expression parser, last published nearly three years ago. Not considered further.

Day-versus-month ambiguity: none of these crates disambiguates by data. `chrono-english` disambiguates by the caller's `Dialect` and `dateparser` by fixed convention. That is the right shape for this project too: the Locale decides, there is no guessing from the digits, and a value like `13/09` is an error in a month-first Locale rather than silently reordered. (The reverse, accepting `09/13/2026` in a day-first Locale by inspecting values above 12, would make `03/09/2026` mean different things for the same user depending on the digits and is a foot-gun in a ledger.)

Verdict: no crate parses Locale-derived numeric dates plus Locale-supplied relative words. `chrono-english`/`interim` come closest for English, but they bring in a large grammar the product does not want (`next friday`), fixed year pivots, non-localisable error strings and a `DateTime` base. The needed grammar is small enough (three numeric fields, ISO, three words) that a derived hand-written parser over `chrono::NaiveDate::from_ymd_opt` is smaller and more predictable than adapting either crate.

## 4. Relative words from a Message

- Fluent has no built-in "list of alternative spellings", so the words are modelled as one Message per word whose value is the Locale's word: `date-word-today = today`, `date-word-yesterday = yesterday`, `date-word-tomorrow = tomorrow`. A translator for another Locale supplies the local word (for example `hoy`, `ayer`, `mañana`); an author may add a second accepted spelling as further Messages (`date-word-today-alt`) or as a comma-free single value per Message to keep parsing trivial.
- "English always accepted" is implemented in code, not in the FTL: the parser's accepted set is a built-in `[("today", 0), ("yesterday", -1), ("tomorrow", 1)]` table unioned with the active Locale's Message values (resolved once when the Locale changes, or on each parse, via `FluentBundle::get_message` and `format_pattern`). The built-in table means a missing translation cannot make `today` stop working, and the same words are used for the English fallback bundle.
- Matching: trim, then compare with `str::to_lowercase()` on both sides (Unicode lower-casing, no Locale-specific casing; that is enough for the English words and the Latin-script Locales in view). Do not use `icu_casemap` unless a Locale with special casing (Turkish dotless i) is added. No fuzzy matching or prefix matching (`tod`), so the accepted set is explicit and testable.
- The hint under the field (section 5) should list the accepted words from the same source, so the message and the parser cannot drift.
- Relative words are resolved against an injected `today: NaiveDate`, as `parse_date` already does, so tests are deterministic and time zones stay the caller's decision.

## 5. Year-less and two-digit-year input, and errors as Messages

**Two-digit years.** `en-AU` and `en-US` short patterns print `yy` (measured above), so a value the app itself displayed (`3/9/26`) must be accepted, or typed input would not round-trip the display. Recommendation: accept 1, 2 or 4 digit years; for exactly two digits, use `chrono`'s own `%y` rule (70 to 99 is 19xx, 00 to 69 is 20xx) to stay consistent with the storage library, or a window centred on today (within 50 years back and 49 forward) if a ledger with future-dated entries (scheduled bookings) matters. The window centred on today is friendlier to both historical statements and forward dates and is a three-line function; the fixed 1940 to 2040 window in `chrono-english` ages badly. One or three digits should be an error (a year of `2` is almost certainly a typo, not the year 2).

**Year-less input.** `3/9` works in `chrono-english` by assuming the current year. Recommendation: accept a year-less numeric input only when it has exactly the day and month fields of the Locale's pattern and treat the year as `today`'s year. For a filter From/To field this is convenient and matches how people type. The hint should say so. It should not be accepted in a Transaction entry form, where a silent wrong year is worse than a rejection; that policy belongs to the call site, so the parser takes an option (`AllowYearless`) rather than deciding.

**Separators.** Accept only the pattern's separator and ISO, so `3-9-26` is an error rather than a guess. Leniency with `-` and `.` would blur the line with ISO; a non-padded ISO such as `2026-9-3` is a reasonable extra accepted form, but that should be a deliberate choice for the implementer.

**Errors as Messages.** `parse_date` should return a typed error with data, not a `String`, and the UI renders a Message:

- `DateParseError::Unrecognised` (shape did not match the pattern, ISO, or a known word)
- `DateParseError::OutOfRange { field }` (the shape matched but `from_ymd_opt` refused, for example day 31 in September or month 13, which is precisely what a day/month mix-up produces)
- `DateParseError::BadYear` (one or three digit year)
- `DateParseError::YearRequired` (year-less input where not allowed)

The Messages then read, for example: `date-error-unrecognised = Use { $example }, { $iso } or { $words }`, with `$example` produced by the same `DateTimeFormatter` pattern (today's date in the Locale's short form, as `parse_date` already does with `format::date`), `$iso` today's ISO date, and `$words` the accepted word list joined by `icu_list`'s list formatter (already in the ICU4X family) so the "or" is a Locale concern. `date-error-out-of-range = { $field } is not a valid date in that month`. Suppress the ISO/style duplicate (the existing code drops the styled example when it equals ISO) by leaving `$example` out when it is identical.

## 6. Recommendation

**Derive the numeric parser from the Locale's resolved short pattern (`FormattedDateTime::pattern()` rendered to a string, read for the order of `d`/`M`/`y` and the separator), always also accept ISO and the built-in English words unioned with the Locale's `date-word-*` Messages, and implement the whole thing as a small hand-written parser in `lib-localisation` over `chrono::NaiveDate::from_ymd_opt`. Do not adopt `chrono-english`, `interim` or `dateparser`.**

- Order and separator come from the pattern (`d/M/yy`, `M/d/yy`, `dd/MM/y` for the three Locales), so parsing accepts exactly what formatting prints and new Locales need no table.
- The Locale is the only disambiguator (no digit-driven reordering), so `03/09/2026` is unambiguous per Locale, and `13/09` in a month-first Locale is `OutOfRange`.
- Relative words: three Messages plus an always-present English table; resolved against an injected `today`.
- Two-digit years accepted (they are what `en-AU`/`en-US` print), with a window centred on today; year-less input as an opt-in for filter fields only.
- Errors are a typed enum mapped to Messages, with the example and word list rendered through the same formatter and list formatter.

**Tradeoffs.** Pros: no new dependency, one source of truth with the formatter, translator-owned words, testable and small (the current three hand parsers are already about 30 lines; the derived one replaces them). Cons: reading the pattern from its string ties the parser to UTS 35 syntax (stable, documented, but the string route is the public one because `iter_items` is `pub(crate)`); month-name input (`3 Sep 2026`) needs a separate small table or is dropped from typed input (recommend dropping it from the default and keeping only ISO, numeric and words, since it was a `DateFormat` variant that ADR-0021 removes); the pattern read needs an assumption that the first three field letters are `d`, `M` and `y`, so a Locale with a different or exotic short pattern must fall back to ISO plus words with a clear error rather than mis-parse.

**Fallback if reading the pattern string is unwelcome:** a table of (order, separator) per Locale, in three rows for `en-AU`/`en-US`/`en-GB`, in the same parser; the parser and Messages are unchanged.

## 7. Sketch

```rust
// lib-localisation/src/date_input.rs (illustrative, not built)
use chrono::NaiveDate;

pub enum Field { Day, Month, Year }

/// Read from `FormattedDateTime::pattern().to_string()`, e.g. "d/M/yy" or "dd/MM/y".
pub struct ShortShape { order: [Field; 3], separator: char }

impl ShortShape {
    pub fn from_pattern(pattern: &str) -> Option<Self> {
        let mut order = Vec::new();
        let mut separator = None;
        for ch in pattern.chars() {
            match ch {
                'd' if !matches!(order.last(), Some(Field::Day)) => order.push(Field::Day),
                'M' if !matches!(order.last(), Some(Field::Month)) => order.push(Field::Month),
                'y' if !matches!(order.last(), Some(Field::Year)) => order.push(Field::Year),
                'd' | 'M' | 'y' => {}
                other if !other.is_alphabetic() => separator = separator.or(Some(other)),
                _ => return None,
            }
        }
        Some(Self { order: order.try_into().ok()?, separator: separator? })
    }
}

pub enum DateParseError { Unrecognised, OutOfRange(Field), BadYear, YearRequired }

pub fn parse_date(
    text: &str,
    today: NaiveDate,
    shape: &ShortShape,
    words: &[(String, i64)], // English built-ins plus the Locale's date-word-* Messages
    allow_yearless: bool,
) -> Result<Option<NaiveDate>, DateParseError> {
    let text = text.trim().to_lowercase();
    if text.is_empty() { return Ok(None); }
    if let Some((_, offset)) = words.iter().find(|(word, _)| *word == text) {
        return Ok(today.checked_add_signed(chrono::Duration::days(*offset)));
    }
    if let Some(date) = parse_iso(&text) { return Ok(Some(date)); }
    // split on shape.separator; two parts => year-less if allowed, three => full;
    // map parts through shape.order; expand two-digit years around `today`;
    // NaiveDate::from_ymd_opt failure => OutOfRange (the day/month mix-up case).
    todo!()
}
```

The UI maps `DateParseError` to Messages (`date-error-unrecognised`, `date-error-out-of-range`, and so on), passing the formatted example and the word list as arguments.

## 8. Open points for the wider map

- Whether month-name typed input survives at all once `DateFormat::DayMonthYear` (`01 jan 2026`) is retired; this note recommends dropping it, which the Preference reconciliation ticket should confirm.
- Which two-digit-year rule to adopt (chrono's 70 pivot for consistency, or the window centred on today); a product decision rather than a technical one.
- Whether year-less input is allowed per call site (filters yes, entry forms no) and whether the hint should show it.
- The `en-XA` pseudo-Locale: its date shape and words should fall back to `en-AU`, mirroring the fallback decision in the formatting note.
- Whether the TUI's future date inputs share this parser (they should, via `lib-localisation`), since the TUI had no typed date parser in the files searched.
