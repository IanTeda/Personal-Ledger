-- Migration: create budget_accounts table -- the Accounts whose Splits a Budget counts. A Budget's
-- Accounts are rewritten whenever it is, so they carry no timestamp column, and read back in rowid
-- order, the order they were written in.
--
-- Deleting an Account takes it from every Budget (ON DELETE CASCADE), so no Budget keeps a
-- deleted Account's id in the file (#952).

CREATE TABLE IF NOT EXISTS budget_accounts (
    budget_id UUID NOT NULL REFERENCES budgets(id) ON DELETE CASCADE,
    account_id UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    PRIMARY KEY (budget_id, account_id)
);

CREATE INDEX IF NOT EXISTS idx_budget_accounts_account_id ON budget_accounts(account_id);
