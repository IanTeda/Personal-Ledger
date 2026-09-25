# Units

A Unit is the denomination you use to measure value in an account. Your bank account is in AUD (Australian dollars). Your Bitcoin wallet is in BTC. Your Apple share holding is in AAPL. Personal Ledger tracks all of them the same way: one Unit per account, set when you create it and fixed forever.

## What you can do

- Create units for currencies, cryptocurrencies, stocks, ETFs, commodities and custom assets
- Assign a unit to each account at creation
- View and manage your holdings in each unit
- Set optional pricing information for valuations
- Track multiple units without cross-unit conversion

## Terminology

- **Unit:** The denomination or asset class an account is denominated in (e.g. AUD, BTC, AAPL).
- **Unit Code:** A unique uppercase identifier (e.g. AUD, BTC, AAPL).
- **Unit Type:** The category of unit (Currency, Crypto, Equity, ETF, Commodity, Custom).
- **Pricing Pair:** The unit you use to price another unit (e.g. BTC priced in AUD).
- **Precision:** The number of decimal places supported (e.g. AUD uses 2, BTC uses 8).

## Units concept

In Personal Ledger, every account holds a single unit from the moment you create it. That unit is fixed and cannot be changed later. Your bank account is always AUD. Your Bitcoin wallet is always BTC. Your Apple shares are always AAPL.

A unit can represent a fiat currency (AUD, USD, EUR), a cryptocurrency (BTC, ETH), a listed stock (AAPL, BHP), an ETF (VAS, IVV), a commodity (XAU for gold, XAG for silver), or anything else you define. Personal Ledger treats all units uniformly — the same storage and calculation rules apply to currencies as to crypto or stocks.

### Why units are fixed per account

An account's unit defines what its balance means. If you decide "this account is in AUD" and later change your mind, every transaction amount, balance calculation, and report would need recalculation. Keeping it fixed makes those calculations reliable and history unambiguous.

## Approach

When you create an account, choose a unit. If the unit already exists in your ledger (from another account), select it. If it's new, create it with a name and type (currency, crypto, stock, etc.). You can optionally set its precision (how many decimal places it uses — AUD uses 2, Bitcoin uses 8) and a pricing pair if you want valuations (e.g. "price Bitcoin in AUD").

## Worked example

You create your first account, an everyday bank account, and choose AUD as the unit — you're in Australia and use Australian dollars. You create a second account for your Bitcoin holdings and choose BTC as the unit, setting precision to 8 decimal places (Bitcoin's standard). You create a third account for Apple shares and choose AAPL as the unit, setting precision to 6. These three accounts now hold completely separate balances in completely separate units. Personal Ledger does not automatically convert between them or show a single "total" — each unit's balance stands alone.

## Types of units

Personal Ledger supports currencies (AUD, USD, EUR), cryptocurrencies (BTC, ETH), equities (AAPL, BHP), ETFs (VAS, IVV), commodities (XAU for gold, XAG for silver), and custom units for assets that don't fit predefined categories (employee share schemes, loyalty points, store credit).

## Screens

The Units screen lists all units you've created with their names, types, precision, and whether they're currently in use by an account. In the terminal app, press `g` then `u` to jump there. The Units screen in the desktop app is still in design.

## Rules to know

- A unit, once created and assigned to an account, cannot be changed.
- Cross-unit transactions (moving money between accounts in different units) require manual exchange-rate tracking in a transaction split — they are not supported directly.
- Precision (decimal places) is set when creating a unit and controls value entry and display.
- Inactive units are kept if they have historical transactions associated with them.

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

- [x] UNT-001 (TUI, desktop): Create a unit with a code, name, type (Currency, Crypto, Equity, ETF, Commodity, Custom), and precision
- [x] UNT-002 (TUI, desktop): List units with filtering by active status
- [x] UNT-003 (TUI, desktop): Edit a unit's name and active status
- [x] UNT-004 (TUI, desktop): Set a pricing pair for optional valuation (AUD, USD, etc.)
- [x] UNT-005 (TUI, desktop): Deactivate a unit without deleting it if it has transactions
- [ ] UNT-006: Bulk import units from standard currency/commodity/crypto lists
- [ ] UNT-007: Market pricing integration and valuation calculations
- [ ] UNT-008: Cross-unit aggregate reporting and portfolio valuation

## For developers

Curious how units are structured in the codebase, or planning to change them? See the [Units development documentation](development/units.md).
