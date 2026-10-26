-- Migration: create payees and payee_aliases tables -- Payee is a first-class entity with
-- rename aliases (ADR-0012). Created before `splits` and `bill_plans`, which reference
-- `payees(id)`.
--
-- default_category_id is the leaf Category that pre-fills a new Split for the Payee. It references
-- the Category with no cascade: Category delete clears the defaults naming it first, and the key
-- is the backstop to that (#858).

CREATE TABLE IF NOT EXISTS payees (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    default_category_id UUID REFERENCES categories(id),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TRIGGER IF NOT EXISTS trg_payees_set_updated_on
AFTER UPDATE ON payees
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE payees
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;

-- A Payee Alias (the UI's "match rule"): a former name or a hand-authored rule, stored trimmed and
-- upper-cased. pattern is unique across Payees (ADR-0012 as amended), and an alias goes with its
-- Payee when the Payee is deleted. A Payee's aliases are rewritten whenever it is, so they carry
-- no timestamp column, and read back in rowid order, the order they were written in.
CREATE TABLE IF NOT EXISTS payee_aliases (
    id UUID PRIMARY KEY,
    payee_id UUID NOT NULL REFERENCES payees(id) ON DELETE CASCADE,
    pattern TEXT NOT NULL UNIQUE
);

CREATE INDEX IF NOT EXISTS idx_payee_aliases_payee_id ON payee_aliases(payee_id);
