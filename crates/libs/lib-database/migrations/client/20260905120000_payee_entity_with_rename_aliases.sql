-- Migration: promote Payee to a first-class entity with rename aliases (ADR-0012).
--
-- `payees`/`payee_aliases` are new. `transactions.payee` (free TEXT) is replaced with
-- `payee_id`, a nullable foreign key to `payees.id` -- SQLite requires a table rebuild to
-- change a column, not just ALTER COLUMN. No SQL-only backfill of any existing free-text
-- payee value: a RowID requires a genuine UUIDv7 (see lib_core::RowID), which plain SQL
-- cannot generate, so a local dev Transaction predating this migration simply loses its
-- Payee link -- there is no real user data to preserve yet (see ADR-0012). New Transactions
-- resolve or auto-create their Payee going forward.

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
-- happened (the same reasoning `transactions` already uses to omit created_on, FR.21).
CREATE TABLE IF NOT EXISTS payee_aliases (
    id UUID PRIMARY KEY,
    payee_id UUID NOT NULL REFERENCES payees(id),
    pattern TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_payee_aliases_payee_id ON payee_aliases(payee_id);

-- Rebuild transactions: payee TEXT -> payee_id (nullable FK to payees.id).
CREATE TABLE transactions_new (
    id UUID PRIMARY KEY,
    date TEXT NOT NULL,
    amount TEXT NOT NULL,
    category_id UUID NOT NULL REFERENCES categories(id),
    account_id UUID NOT NULL REFERENCES accounts(id),
    payee_id UUID REFERENCES payees(id),
    description TEXT,
    status TEXT NOT NULL DEFAULT 'open',
    is_flagged BOOLEAN NOT NULL DEFAULT FALSE,
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

INSERT INTO transactions_new (id, date, amount, category_id, account_id, payee_id, description, status, is_flagged, updated_on)
SELECT id, date, amount, category_id, account_id, NULL, description, status, is_flagged, updated_on
FROM transactions;

DROP TABLE transactions;
ALTER TABLE transactions_new RENAME TO transactions;

CREATE INDEX IF NOT EXISTS idx_transactions_account_id ON transactions(account_id);
CREATE INDEX IF NOT EXISTS idx_transactions_category_id ON transactions(category_id);
CREATE INDEX IF NOT EXISTS idx_transactions_payee_id ON transactions(payee_id);

-- Dropping the old `transactions` table also dropped its trigger; recreate it.
CREATE TRIGGER IF NOT EXISTS trg_transactions_set_updated_on
AFTER UPDATE ON transactions
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE transactions
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;
