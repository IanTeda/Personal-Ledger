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

## Amendment: how fetching writes Weekly Prices

Decided across the [CORE: Units price themselves from their Price Sources](https://github.com/IanTeda/Personal-Ledger/issues/1367) map, recorded by [CORE: amend ADR-0043 and ADR-0052 for Units that price themselves](https://github.com/IanTeda/Personal-Ledger/issues/1381). Weeks stay the only stored grain.

- **Sync:** a fetched Weekly Price is an ordinary Unit write and syncs whenever Unit writes do, closing the open Consequence above. Which Client fetches when several are open belongs to the sync effort.
- **Fetching:** a fetch, however often, writes the week of the Clock's day through `record_price`: it overwrites the provisional close and widens the high and low ([ADR-0043](0043-built-in-price-sources-are-code-referenced-by-id-and-api-keys-sync-encrypted.md)'s "where and when fetching runs").
- **Backfill:** a Unit on a fetching source fills only its empty past weeks, back to its Accounts' earliest Opened date, each folded from daily points; it never overwrites a week that has a price, and Weekly Prices gain no provenance column (ADR-0043's "backfilling a Unit's history").
- **Finer prices are never stored:** Daily and Hourly Prices are fetched for the Unit prices view to show and cached in memory only, which keeps "Every fetched price" rejected.
