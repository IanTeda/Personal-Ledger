-- Migration: create ledger table -- the one row of Ledger-wide state, holding the base Unit
-- every other Unit's price is quoted in unless a person picks another (ADR-0044). The
-- singleton is a database constraint here (id = 1), unlike preferences, since the row needs
-- no UUIDv7.

CREATE TABLE IF NOT EXISTS ledger (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    base_unit_id UUID NOT NULL REFERENCES units(id)
);
