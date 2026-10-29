# Units keep a Weekly Price history

ADR-0043 and the Units functionality map (#667) kept a Unit's link to "symbol, quote and last price only", with no history. Grilling the Units view model gaps the TUI found (#765) reversed that: a Unit keeps a **Weekly Price** for every ISO week (Monday to Sunday, keyed by its Monday, the same in every Locale) in its quote Unit, a close with an optional open, high and low, kept forever. The current week's entry is provisional, overwritten by each new price and final when the week ends, and the Unit's last price is the newest entry's close rather than a field of its own. The quote Unit owns the history: changing it clears every Weekly Price, while changing the Price Source, Manual entry included, keeps them.

The Units page wants a weekly close chart, a weekly prices table, the week's change and the 52-week range, and Manual entry Units need somewhere for a person's own prices to accumulate. One row per Unit per week stays small (about 52 a year) while carrying enough for all of these.

## Considered Options

- **Last price only (ADR-0043 as written).** Rejected: every history figure the TUI drew before the VIPER move would be dropped for good.
- **Every fetched price.** Rejected: a source fetching hourly grows the Ledger, and its sync, by thousands of rows a year per Unit for figures that only ever read weekly.
- **Weeks starting on the Locale's first day.** Rejected: one Ledger would bucket the same prices differently on two machines.
- **Open, high, low and close all required.** Rejected: Manual entry usually has a close only. A week with all four draws as a candle, any other as a close tick.
- **A rolling window trimmed to a year.** Rejected: it would throw away a person's own Manual entry history to save rows that cost little.

## Consequences

- `last_price` on `UnitRow` gives way to Weekly Prices, persisted in a new `unit_prices` table in `migrations/client/`.
- How a fetched Weekly Price syncs stays with Price Source fetching, as ADR-0043 left it.
