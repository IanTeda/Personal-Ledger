-- Migration: rename accounts -> sync_users
--
-- Fixes a naming collision surfaced while resolving "Decide the TUI app's data model
-- and persistence layer" (CC-TUI-004): this table is the Sync Server's own single-user
-- auth credential (ADR-0010), not the domain Account entity (Cash/Bank/Credit
-- Card/Investment/Loan) the Ledger uses that name for. See CONTEXT.md's SyncUser
-- glossary entry.

ALTER TABLE accounts RENAME TO sync_users;

DROP TRIGGER IF EXISTS trg_accounts_set_updated_on;

CREATE TRIGGER IF NOT EXISTS trg_sync_users_set_updated_on
AFTER UPDATE ON sync_users
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE sync_users
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;
