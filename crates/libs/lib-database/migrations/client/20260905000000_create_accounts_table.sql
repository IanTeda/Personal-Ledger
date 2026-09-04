-- Migration: create accounts table -- the domain Account entity (Cash/Bank/Credit
-- Card/Investment/Loan, FR.10-15, CC-TUI-008). Not to be confused with the Sync Server's
-- own auth credential, renamed to sync_users to free this name up (see
-- 20260904000000_rename_accounts_to_sync_users.sql and CONTEXT.md's SyncUser entry).
--
-- unit_id is the first real foreign key in the schema: an Account's Unit is fixed at
-- creation (FR.10) and must reference a real Unit. Enforced by SQLite's foreign_keys
-- pragma, turned on for every connection in lib-database's DatabaseConnection::new.

CREATE TABLE IF NOT EXISTS accounts (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    account_type TEXT NOT NULL,
    unit_id UUID NOT NULL REFERENCES units(id),
    starting_balance TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_accounts_unit_id ON accounts(unit_id);


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
