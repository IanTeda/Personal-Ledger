-- Migration: create transactions table -- single-entry Transactions against exactly one
-- Account on a date (FR.16-21, CC-TUI-009). Its amounts, Categories and Payees are its Splits'
-- (the splits table); a Transaction's total is their sum, never stored. No created_on column:
-- FR.21 says the UUIDv7 id itself determines creation date, the same convention Units already
-- uses.
--
-- account_id has no foreign key yet: Accounts aren't persisted, so no accounts row would back it
-- (#774). It becomes REFERENCES accounts(id) when the Accounts domain persists.

CREATE TABLE IF NOT EXISTS transactions (
    id UUID PRIMARY KEY,
    date TEXT NOT NULL,
    account_id UUID NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'cleared', 'reconciled')),
    is_flagged BOOLEAN NOT NULL DEFAULT FALSE,
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_transactions_account_id ON transactions(account_id);

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
