-- Migration: create budget_limit_records table -- one record of a Category Limit chain: what
-- starts in month for one leaf Expense Category in one Budget (ADR-0028). A chain holds at most
-- one record per month, which the primary key holds. A Budget's records are rewritten whenever it
-- is, so they carry no timestamp column.
--
-- month is the first day of its calendar month. kind is Onward (from its month until the next
-- record), Month-only (its month alone) or Stop (no amount from its month); amount is a decimal
-- string, null exactly for a Stop, and rollover is 'none' for a Stop.
--
-- category_id references the Category with no cascade: Category delete clears the chains naming
-- it first, and the key is the backstop to that (#952).

CREATE TABLE IF NOT EXISTS budget_limit_records (
    budget_id UUID NOT NULL REFERENCES budgets(id) ON DELETE CASCADE,
    category_id UUID NOT NULL REFERENCES categories(id),
    month TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('onward', 'month_only', 'stop')),
    amount TEXT CHECK ((kind = 'stop') = (amount IS NULL)),
    rollover TEXT NOT NULL CHECK (rollover IN ('none', 'carry_unspent', 'carry_both')),
    PRIMARY KEY (budget_id, category_id, month)
);

CREATE INDEX IF NOT EXISTS idx_budget_limit_records_category_id
    ON budget_limit_records(category_id);
