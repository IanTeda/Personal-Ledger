-- Migration: create transactions table -- single-entry Transactions against exactly one
-- Account and one Category (FR.16-21, CC-TUI-009). No created_on column: FR.21 says the
-- UUIDv7 id itself determines creation date, the same convention Units already uses.
--
-- Both foreign keys are enforced by SQLite's foreign_keys pragma (see DatabaseConnection::
-- new). Moving a Transaction to a different-Unit Account (rejected by FR.19) can't be
-- expressed as a schema constraint -- it's an application-level check in
-- Transactions::update, since SQLite FKs can't compare two rows' Unit columns to each other.

CREATE TABLE IF NOT EXISTS transactions (
    id UUID PRIMARY KEY,
    date TEXT NOT NULL,
    amount TEXT NOT NULL,
    category_id UUID NOT NULL REFERENCES categories(id),
    account_id UUID NOT NULL REFERENCES accounts(id),
    payee TEXT,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'open',
    is_flagged BOOLEAN NOT NULL DEFAULT FALSE,
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_transactions_account_id ON transactions(account_id);
CREATE INDEX IF NOT EXISTS idx_transactions_category_id ON transactions(category_id);


-- Trigger to update updated_on on every transaction row change
CREATE TRIGGER IF NOT EXISTS trg_transactions_set_updated_on
AFTER UPDATE ON transactions
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE transactions
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;
