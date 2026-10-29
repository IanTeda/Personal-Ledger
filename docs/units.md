# Units

A Unit is the denomination you use to measure value in an account. Your bank account is in AUD (Australian dollars). Your Bitcoin wallet is in BTC. Your Apple share holding is in AAPL. Personal Ledger tracks all of them the same way: one Unit per account, set when you create it and fixed forever.

## What you can do

- Create units for currencies, crypto, equities, fixed income, commodities and custom assets
- Assign a unit to each account at creation
- View and manage your holdings in each unit
- Set optional pricing information for valuations
- Set the price of a unit you price by hand, for this week or an earlier one
- See a unit's weekly prices: a chart of its weekly closes over a table of each week's prices, a year at a time
- Track multiple units without cross-unit conversion

## Terminology

- **Unit:** The denomination or asset class an account is denominated in (e.g. AUD, BTC, AAPL).
- **Unit Code:** A unique uppercase identifier (e.g. AUD, BTC, AAPL).
- **Unit Kind:** The fixed grouping a unit carries, one of six: Currency, Crypto, Equity, Fixed Income, Commodity or Custom. It also decides which Price Sources can price the unit.
- **Price Source:** Where a unit's price comes from: a built-in source (CoinGecko, Yahoo Finance, Frankfurter), a custom source you add, or Manual entry, where you type the price yourself.
- **Base Unit:** The one unit your ledger counts in. It is marked ◆ on the Units page, and new accounts start on it. There is no separate default unit. Change it with the *Base unit* field on the General page, or with Set base unit on the Units page (`B`, in both the Desktop and the TUI; `units.set_base` under `[keybindings]` moves it). There is always one: deleting the base unit asks which other unit takes over, and the base unit cannot be deleted when it is your only unit.
- **Pricing Pair:** The unit you use to price another unit (e.g. BTC priced in AUD).
- **Precision:** The number of decimal places a unit's amounts show at, from 0 to 18 (e.g. AUD uses 2, BTC uses 8). A price shows at the precision of the unit it is quoted in, since it is an amount in that unit.

## Units concept

In Personal Ledger, every account holds a single unit from the moment you create it. That unit is fixed and cannot be changed later. Your bank account is always AUD. Your Bitcoin wallet is always BTC. Your Apple shares are always AAPL.

A unit can represent a fiat currency (AUD, USD, EUR), a cryptocurrency (BTC, ETH), a listed stock (AAPL, BHP), an ETF (VAS, IVV), a commodity (XAU for gold, XAG for silver), or anything else you define. Personal Ledger treats all units uniformly — the same storage and calculation rules apply to currencies as to crypto or stocks.

### Why units are fixed per account

An account's unit defines what its balance means. If you decide "this account is in AUD" and later change your mind, every transaction amount, balance calculation, and report would need recalculation. Keeping it fixed makes those calculations reliable and history unambiguous.

## Approach

When you create an account, choose a unit; the form starts on your base unit. If the unit already exists in your ledger (from another account), select it. If it's new, create it with a name and type (currency, crypto, stock, etc.). Its precision (how many decimal places it uses — AUD uses 2, Bitcoin uses 8) starts on the type's, which you can change, and you can optionally set a pricing pair if you want valuations (e.g. "price Bitcoin in AUD").

## Worked example

You create your first account, an everyday bank account, and choose AUD as the unit — you're in Australia and use Australian dollars. You create a second account for your Bitcoin holdings and choose BTC as the unit, setting precision to 8 decimal places (Bitcoin's standard). You create a third account for Apple shares and choose AAPL as the unit, setting precision to 6. These three accounts now hold completely separate balances in completely separate units. Personal Ledger does not automatically convert between them or show a single "total" — each unit's balance stands alone.

## Types of units

Every unit is one of six Unit Kinds:

- **Currency:** fiat money, such as AUD, USD or EUR.
- **Crypto:** coins and tokens, stablecoins included, such as BTC, ETH or USDC.
- **Equity:** shares, ETFs, LICs, REITs, managed funds and employee shares or options, such as BHP or VAS.
- **Fixed Income:** bonds and other market-priced debt. A term deposit isn't a unit: it's AUD held in an account.
- **Commodity:** precious metals and other raw goods, such as XAU for gold.
- **Custom:** anything else you track, such as loyalty points, store credit or collectibles.

The Unit Kind decides which built-in Price Sources are offered for a unit: CoinGecko for Crypto, Yahoo Finance for Equity, Fixed Income and Commodity, and Frankfurter for Currency. Manual entry and any custom source you add are offered for every kind.

## Screens

The Units screen lists all units you've created with their names and types, each unit's last price in the unit it is quoted in (and how long ago it was written: under the price in the desktop app, beside it in the terminal app's summary for the selected unit), and whether they're currently in use by an account. At its widest, the detail pane shows each account's balance in the unit at the unit's precision, and its last price and the accounts' values at the precision of the unit it is quoted in. In the terminal app, press `g` then `u` to jump there. The Units screen in the desktop app is still in design.

## Rules to know

- A unit that an account, a bill plan, a budget or a location uses cannot be deleted. In the TUI, neither can a unit an institution names as its usual unit. The Delete unit dialog opens as *Deactivate unit*, saying what uses it, without asking you to type the code first. Edit unit shows the same counts (*Used by 2 accounts, 1 budget*). A deactivated unit stays on the Units page, dimmed and tagged *inactive*, and stays on whatever uses it, but it is hidden from the Unit picker when you add an account and from the *Base unit* field on the General page. Reactivate it from its row menu, the detail pane or by pressing `r` on the row.
- In the Desktop, a unit that institutions name as their usual unit can still be deleted. The Delete unit dialog lists each of those institutions with a *Usual unit* select, starting on *None*, that offers every other active unit. Choose the unit each institution uses instead, or leave it on *None* to leave it with no usual unit; it never switches to the base unit on its own. When the unit is also the base unit, the same dialog asks which unit takes over as base above them. Move between the selects with `Tab` and `Shift-Tab` and change the one you're on with `Up` and `Down`, or click them. Type the code and confirm: the unit goes and the institutions take their new usual units together, or nothing changes. Cancelling changes nothing.
- A unit that other units are quoted in cannot be deleted, and *Deactivate unit* is not offered for it. The Delete unit dialog says how many units are quoted in it, for example *aud is the quote unit of 2 other units*. Change those units' *Quoted in* field first (Edit unit), then delete it. That includes the base unit: delete it only once no other unit is quoted in it.
- The base unit cannot be deactivated: its *Deactivate unit* button is disabled, with the reason beside the counts. Pick another base unit on the General page first. An unused base unit that no other unit is quoted in can still be deleted by choosing the unit that takes over.
- A unit, once created and assigned to an account, cannot be changed.
- Cross-unit transactions (moving money between accounts in different units) require manual exchange-rate tracking in a transaction split — they are not supported directly.
- Precision is the *Precision* field of Add unit and Edit unit, a whole number from 0 to 18. Add unit starts it at 8 for Crypto and 2 for every other type, and follows the type you pick until you type a precision yourself; Edit unit never changes it for you. Changing a unit's precision changes only how its amounts show: they are rounded for display and kept as stored. A price is shown at its quote unit's precision, so a share priced under a dollar to three places, as some on the ASX are, shows to two places in AUD.
- Inactive units are kept if they have historical transactions associated with them.
- A unit on Manual entry is priced only by you. Select it and press `p`, or choose *Set price…* from its row menu or *Set price* in the detail pane. *Week* starts on today and takes any day of the week you mean, typed in your locale's short date form or as `YYYY-MM-DD`; the dialog shows the Monday that week starts on. *Close* is in the unit it is quoted in, at that unit's precision. Setting this week's price again overwrites it and widens the week's high and low; setting an earlier week replaces whatever that week had. A week that hasn't started yet is refused. A unit priced by a fetching source doesn't offer Set price.
- A unit's prices open from the Units table: select the unit and press `Enter`, or choose *Prices* from its row menu or the detail pane. The view repeats the unit's price figures (its last price and how long ago it was written, the change on the week before, its 52-week range, and its symbol and the unit it is priced in), then charts a year of weekly closes, a candle for a week with an open, high and low and a tick for a week with only a close, over a table of the same weeks, newest first, with each week's open, high, low, close and its change on the week before. Press `[` to look a month further back and `]` to come a month forward again, no further than this week. On a unit you price by hand, `p` sets a price from here too. `Esc` takes you back to the Units table with the same unit selected. A unit with no prices yet says so.

## Scope

- Cross-unit aggregation (showing a total across multiple units) is not supported — each unit's balance stands alone.
- Automatic pricing and valuation (market value, unrealised gains) are future considerations.
- Exchange rate workflows and multi-unit reporting are future considerations.
- Bulk unit creation from currency/commodity lists is not built yet.

## Related

- **Accounts:** each account is denominated in exactly one unit.
- **Transactions:** amounts are recorded in the account's unit.
- **Reports:** each report covers one unit at a time.

## Getting around

| Go to | Terminal app | Desktop app |
| --- | --- | --- |
| Units | `g` `u` | not in the desktop app yet |

For the full set of keys, see [Getting around](getting-around.md).

## Feature set and requirements

Intended features for Units. Ticked means built in at least one app; the tag says which.

- [x] UNT-001 (TUI, desktop): Create a unit with a code, name, Unit Kind (Currency, Crypto, Equity, Fixed Income, Commodity, Custom), and precision
- [x] UNT-002 (TUI, desktop): List units with filtering by active status
- [x] UNT-003 (TUI, desktop): Edit a unit's name, precision and active status
- [x] UNT-004 (TUI, desktop): Set a pricing pair for optional valuation (AUD, USD, etc.)
- [x] UNT-005 (TUI, desktop): Deactivate a unit without deleting it if it has transactions
- [ ] UNT-006: Bulk import units from standard currency/commodity/crypto lists
- [ ] UNT-007: Market pricing integration and valuation calculations
- [ ] UNT-008: Cross-unit aggregate reporting and portfolio valuation
- [x] UNT-009 (TUI, desktop): Set a Manual entry unit's price for the current or an earlier week
- [x] UNT-010 (TUI, desktop): View a unit's weekly prices as a chart and a table, a year at a time, stepped back and forward by a month

## For developers

Curious how units are structured in the codebase, or planning to change them? See the [Units development documentation](development/units.md).
