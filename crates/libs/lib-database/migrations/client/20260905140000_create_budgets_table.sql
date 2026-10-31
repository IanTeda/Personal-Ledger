-- Migration: create budgets table -- a named container of Category Limits over one Ledger
-- (GLOSSARY.md's Budget, ADR-0028, ADR-0029). Its on-budget Accounts and its Category Limit
-- chains are in budget_accounts and budget_limit_records. A Budget is archived, never deleted.
--
-- method is the Budget's Method; only Category limits is built. unit_id is locked at creation and
-- references the Unit with no cascade: a Unit a Budget names can't be deleted, which the Units
-- usage count refuses first and the key backstops (#952). One Budget is the default, which the
-- partial unique index holds; a write that moves the default unmarks the old one first.

CREATE TABLE IF NOT EXISTS budgets (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    method TEXT NOT NULL CHECK (method IN ('limits')),
    unit_id UUID NOT NULL REFERENCES units(id),
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    archived_on TEXT,
    created_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_budgets_unit_id ON budgets(unit_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_budgets_one_default ON budgets(is_default) WHERE is_default;

CREATE TRIGGER IF NOT EXISTS trg_budgets_set_updated_on
AFTER UPDATE ON budgets
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE budgets
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;
