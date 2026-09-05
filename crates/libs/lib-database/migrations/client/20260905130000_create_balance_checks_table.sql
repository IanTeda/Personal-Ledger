-- Migration: create balance_checks table -- point-in-time assertions of what an Account's
-- Balance should be (FR.28-32, CC-TUI-013). Manual entry only; CSV import (FR.33) is a
-- separate ticket (CC-TUI-012).
--
-- No uniqueness constraint on (account_id, date): nothing in the FRs asks for one, and a
-- future CSV import may legitimately want more than one entry for the same day.

CREATE TABLE IF NOT EXISTS balance_checks (
    id UUID PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES accounts(id),
    date TEXT NOT NULL,
    asserted_balance TEXT NOT NULL,
    created_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_balance_checks_account_id ON balance_checks(account_id);

CREATE TRIGGER IF NOT EXISTS trg_balance_checks_set_updated_on
AFTER UPDATE ON balance_checks
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE balance_checks
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;
