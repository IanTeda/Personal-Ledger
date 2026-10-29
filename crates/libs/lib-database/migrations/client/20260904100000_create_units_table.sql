-- Migration: create units table -- currencies, cryptocurrencies, stocks, precious metals,
-- and other tradeable instruments Accounts and Transactions are denominated in (FR.1-3,
-- CC-TUI-005). A Unit carries its Price Source link (source id, symbol, quote Unit) (ADR-0043,
-- ADR-0044); its prices are Weekly Prices in unit_prices (ADR-0052). The quote Unit is the base
-- Unit unless a person picks another, and the base Unit quotes itself. price_source_id has no FK:
-- Manual entry and a built-in nobody has renamed have no price_sources row (see that table).

CREATE TABLE IF NOT EXISTS units (
    id UUID PRIMARY KEY,
    code TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    unit_kind TEXT NOT NULL CHECK (unit_kind IN
        ('currency', 'crypto', 'equity', 'fixed_income', 'commodity', 'custom')),
    price_source_id TEXT NOT NULL,
    price_symbol TEXT NOT NULL,
    quote_unit_id UUID NOT NULL REFERENCES units(id),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);


-- Trigger to update updated_on on every unit row change
CREATE TRIGGER IF NOT EXISTS trg_units_set_updated_on
AFTER UPDATE ON units
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE units
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;
