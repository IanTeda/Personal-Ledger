-- Migration: create accounts table -- the domain Account entity (Cash/Bank/Credit
-- Card/Investment/Loan, FR.10-15, GLOSSARY.md's Account, #853). Not to be confused with the Sync
-- Server's own auth credential, which is named sync_users to keep this name free (see GLOSSARY.md's
-- SyncUser entry).
--
-- unit_id references the Unit the Account counts in, fixed at creation (FR.10). institution_id
-- references the Institution holding it, with no cascade: an Institution is never deleted while an
-- Account names it. It is never NULL: a Cash Account names an Institution too, usually the Personal
-- Institution every new Ledger is written with (ADR-0049).
-- balance is the opening balance as a decimal string in the Unit's precision, so no precision is
-- lost to a float; opened_at is a calendar date. account_number is the form's optional field.
-- Rows read back in rowid order, the order they were added in. Enforced by SQLite's foreign_keys
-- pragma, turned on for every connection.

CREATE TABLE IF NOT EXISTS accounts (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    institution_id UUID NOT NULL REFERENCES institutions(id),
    account_type TEXT NOT NULL
        CHECK (account_type IN ('cash', 'bank', 'credit_card', 'investment', 'loan')),
    unit_id UUID NOT NULL REFERENCES units(id),
    balance TEXT NOT NULL,
    opened_at TEXT NOT NULL,
    account_number TEXT,
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_accounts_unit_id ON accounts(unit_id);
CREATE INDEX IF NOT EXISTS idx_accounts_institution_id ON accounts(institution_id);

-- Trigger to update updated_on on every account row change
CREATE TRIGGER IF NOT EXISTS trg_accounts_domain_set_updated_on
AFTER UPDATE ON accounts
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE accounts
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;
