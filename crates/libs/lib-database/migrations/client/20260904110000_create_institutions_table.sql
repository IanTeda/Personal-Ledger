-- Migration: create institutions table -- the banks, brokers and funds that hold a Ledger's
-- Accounts (GLOSSARY.md's Institution, #849). Created before `accounts`, whose institution_id
-- will reference it once Accounts persist (#853).
--
-- name is stored trimmed and unique ignoring case, as lib-institutions' add rule refuses a taken
-- name before any write reaches this table; NOCASE folds ASCII only, which the rule's own folding
-- covers. account_type is the Account type label the Add institution dialog joins, stored as
-- given. Rows read back in rowid order, the order they were added in.

CREATE TABLE IF NOT EXISTS institutions (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    account_type TEXT NOT NULL,
    created_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TRIGGER IF NOT EXISTS trg_institutions_set_updated_on
AFTER UPDATE ON institutions
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE institutions
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;
