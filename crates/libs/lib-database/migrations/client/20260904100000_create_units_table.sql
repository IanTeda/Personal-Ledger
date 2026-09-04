-- Migration: create units table -- currencies, cryptocurrencies, stocks, precious metals,
-- and other tradeable instruments Accounts and Transactions are denominated in (FR.1-3,
-- CC-TUI-005). No exchange-rate/pricing table: FR.2/FR.3's cross-Unit exchange-rate and
-- weekly-price-snapshot machinery is superseded by the "no cross-Unit conversion in V1"
-- decision already recorded in CONTEXT.md's Unit entry and docs/product-requirements.md's
-- Constraints/Future Considerations.

CREATE TABLE IF NOT EXISTS units (
    id UUID PRIMARY KEY,
    code TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    unit_kind TEXT NOT NULL,
    decimal_places INTEGER NOT NULL,
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
