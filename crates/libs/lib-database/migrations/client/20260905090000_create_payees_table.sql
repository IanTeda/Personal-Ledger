-- Migration: create payees and payee_aliases tables -- Payee is a first-class entity with
-- rename aliases (ADR-0012). Created before `transactions`, which references `payees(id)`.

CREATE TABLE IF NOT EXISTS payees (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
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

-- Write-once: an alias is only ever inserted by a rename, never updated afterward, so it
-- carries no separate timestamp column -- its id's UUIDv7 stands in for when the rename
-- happened (the same reasoning `transactions` uses to omit created_on, FR.21).
CREATE TABLE IF NOT EXISTS payee_aliases (
    id UUID PRIMARY KEY,
    payee_id UUID NOT NULL REFERENCES payees(id),
    pattern TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_payee_aliases_payee_id ON payee_aliases(payee_id);
