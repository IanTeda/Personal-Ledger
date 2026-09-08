# Units

In Personal Ledger, a Unit represents the denomination used to measure value. Every Ledger (Account) balance and every Transaction amount is recorded in a Unit.

A Unit may represent:

* A fiat currency (AUD, USD, EUR)
* A cryptocurrency or digital asset (BTC, ETH)
* A listed equity or stock (AAPL, BHP)
* An exchange-traded fund (VAS, IVV)
* A commodity or precious metal (XAU, XAG)
* Or, any other user-defined instrument

The system treats all Units uniformly. From the application's perspective, a Unit is simply an identifier with associated metadata that defines how values are displayed, stored, and priced.

## Relationship to Accounts

Every Account is assigned exactly one Unit when it is created, and that assignment remains immutable.

The Account's Unit:

* Defines the denomination of the Account balance.
* Determines how transaction amounts are interpreted.
* Cannot be changed after Account creation.

For example:

Account	Unit
Everyday Banking	AUD
Brokerage Account	AUD
Bitcoin Wallet	BTC
Apple Shares	AAPL


An Account holding Bitcoin records balances in BTC. An Account holding Apple shares records balances in AAPL units.

## Relationship to Transactions

All Transaction amounts are denominated in the Unit of the destination (incoming) Account. Transfers between Accounts with different Units are supported through an exchange rate recorded in the transactions.

Examples:

* Transfer AUD → AUD: no conversion required.
* Transfer AUD → USD: exchange rate required.
* Transfer AUD → BTC: exchange rate required.
* Transfer BTC → ETH: exchange rate required.

The exchange rate defines the relationship between the source Unit and destination Unit at the time of the transaction.

## Unit Types

Personal Ledger supports Fiat Currencies, Cryptocurrencies or Digital Assets, Stocks or Equities, Exchange Traded Funds (ETFs), Managed or Mutual Funds, Commodities, and free-form or custom types.

### Currencies

Government-issued fiat currencies identified by standard currency codes.

Examples:

* AUD
* USD
* EUR
* GBP
* JPY

### Cryptocurrency or Digital Asset

Blockchain-based digital assets.

Examples:

* BTC
* ETH
* SOL
* ADA

### Equity or Stock

Shares traded on public or private exchanges.

Examples:

* AAPL
* MSFT
* BHP
* CBA

### Exchange Traded Fund (ETF)

Exchange-traded funds and similar pooled investment securities.

Examples:

* VAS
* IVV
* VGS
* NDQ

### Commodity or Precious Metal

Tradable physical commodities or precious metals.

Examples:

* XAU (Gold)
* XAG (Silver)
* WTI (Crude Oil)

### Custom

User-defined Units for assets that do not fit predefined categories.

Examples:

* Employee Share Scheme Units
* Private Company Shares
* Loyalty Points
* Store Credit
* Reward Tokens

## Unit Metadata

An uppercase Code uniquely identifies each Unit.

Examples:

* AUD
* BTC
* AAPL
* XAU

The Code acts as the primary identifier throughout the system and must be unique.

Each Unit also stores the following metadata.

### Code <String>

A unique uppercase identifier for the Unit.

Examples:

* AUD
* BTC
* AAPL

Rules:

* Must be unique.
* Stored in uppercase.
* Cannot be changed once created.

### Name <String>

The human-readable name of the Unit.

Examples:

* Australian Dollar
* Bitcoin
* Apple Inc.
* Vanguard Australian Shares ETF

### Type <Enum>

The category of Unit.

Allowed values:

* Currency
* Crypto
* Equity
* ETF
* Commodity
* Custom

### Symbol <String>

The symbol used for market pricing or external integrations.

Examples:

* AUD
* BTC
* AAPL
* VAS.AX

This value may differ from the Unit Code where required by pricing providers.

### Pricing Pair <Unit Reference>

Defines the Unit used to value or price this Unit.

Examples:

Unit	Pricing Pair
BTC	AUD
ETH	AUD
AAPL	USD
VAS	AUD


Pricing services and portfolio valuation calculations use this information.

### Precision <UInt>

The number of decimal places supported by the Unit.

Examples:

Unit	Precision
AUD	2
USD	2
BTC	8
ETH	18
AAPL	6


Precision controls:

* Data validation
* Value entry
* Display formatting
* Rounding behaviour

### Status <Enum>

Indicates whether the Unit is available for use.

Values:

* Active - Available for new Accounts and Transactions.
* Inactive - Retained for historical records but unavailable for new use.

Inactive Units are never deleted if they have historical transactions associated with them.

## Valuation and Pricing

Units may optionally be associated with market pricing data.

Where pricing data is available, Personal Ledger can calculate:

* Current market value
* Portfolio value
* Unrealised gains and losses
* Asset allocation

Pricing is always performed relative to the Unit's configured Pricing Pair.

Examples:

* BTC priced in AUD
* AAPL priced in USD
* Gold priced in AUD

Pricing data does not affect a Unit's stored quantity. It is used only for valuation and reporting.

## Design Principles

The Unit model is intentionally generic.

Personal Ledger does not distinguish between currencies, cryptocurrencies, shares, commodities, or other assets when storing balances and transactions. All assets are represented using the same core Unit abstraction.

This provides several benefits:

* Consistent balance calculations.
* Simplified transaction processing.
* Support for both financial and non-financial assets.
* Extensibility without database schema changes.
* Uniform reporting and valuation workflows.

In practice, a Unit is:

A unique identifier that defines how value is measured, displayed, and optionally priced within Personal Ledger.

## Examples

Code	Name	Type	Pricing Pair	Precision
AUD	Australian Dollar	Currency	AUD	2
USD	United States Dollar	Currency	AUD	2
BTC	Bitcoin	Crypto	AUD	8
ETH	Ethereum	Crypto	AUD	18
AAPL	Apple Inc.	Equity	USD	6
VAS	Vanguard Australian Shares ETF	ETF	AUD	6
XAU	Gold	Commodity	AUD	4


This structure lets Personal Ledger represent cash, investments, digital assets, commodities, and custom instruments with a single, consistent model.
