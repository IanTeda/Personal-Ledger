-- Migration: create ledger table -- the one row of Ledger-wide state: the base Unit every other
-- Unit's price is quoted in unless a person picks another (ADR-0044), and the Ledger's own
-- Preferences, its name, its Owner and the month its Financial Year starts in (ADR-0061). The
-- singleton is a database constraint (id = 1), since the row needs no UUIDv7.
--
-- No column has a SQL default: creating a Ledger writes the whole row in its one transaction
-- (ADR-0054), so a default would only hide a write that forgot one. The Units adapter writes
-- base_unit_id and the Ledger adapter the rest, each only its own columns.

CREATE TABLE IF NOT EXISTS ledger (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    base_unit_id UUID NOT NULL REFERENCES units(id),
    -- Trimmed and at most 60 characters by lib_core::LedgerName; never blank.
    name TEXT NOT NULL CHECK (trim(name) <> ''),
    -- NULL for no Owner, never ''.
    owner TEXT,
    financial_year_start INTEGER NOT NULL CHECK (financial_year_start BETWEEN 1 AND 12)
);
