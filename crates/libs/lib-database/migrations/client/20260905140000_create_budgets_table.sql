-- Migration: create budgets table -- a cap on the total amount of Transactions in one
-- (Expense-type) Category over a recurring period (FR.22-27, CC-TUI-010), line-item only.
--
-- category_id/unit_id are fixed at creation (FR.26 only allows changing limit_amount/
-- period/is_active). No uniqueness constraint on (category_id, unit_id): nothing in the FRs
-- asks for one. The Expense-Category-only restriction can't be expressed as a schema
-- constraint -- it's an application-level check in Budgets::insert.

CREATE TABLE IF NOT EXISTS budgets (
    id UUID PRIMARY KEY,
    category_id UUID NOT NULL REFERENCES categories(id),
    unit_id UUID NOT NULL REFERENCES units(id),
    limit_amount TEXT NOT NULL,
    period TEXT NOT NULL DEFAULT 'monthly',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_budgets_category_id ON budgets(category_id);
CREATE INDEX IF NOT EXISTS idx_budgets_unit_id ON budgets(unit_id);

CREATE TRIGGER IF NOT EXISTS trg_budgets_set_updated_on
AFTER UPDATE ON budgets
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE budgets
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;
