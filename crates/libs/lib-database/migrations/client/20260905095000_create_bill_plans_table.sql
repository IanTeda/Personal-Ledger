-- Migration: create bill_plans table -- the recurring definition of an expected payment
-- obligation (GLOSSARY.md's Bill Plan, docs/bills.md). planned_amount is a positive decimal
-- string so no precision is lost to a float. A Bill Plan is never deleted, only deactivated, so
-- every Bill Schedule entry keeps its Plan. generate_from is the earliest due date generation may
-- create, so an edit or a reactivation never backfills due dates already past (ADR-0019).
--
-- unit_id is the only foreign key: category_id, account_id and payee_id have none yet, since
-- Categories, Accounts and Payees aren't persisted, so no row would back them (#774). Each becomes
-- a REFERENCES when its domain persists.

CREATE TABLE IF NOT EXISTS bill_plans (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    category_id UUID NOT NULL,
    unit_id UUID NOT NULL REFERENCES units(id),
    account_id UUID NOT NULL,
    payee_id UUID,
    planned_amount TEXT NOT NULL,
    amount_kind TEXT NOT NULL CHECK (amount_kind IN ('fixed', 'estimated')),
    recurrence TEXT NOT NULL CHECK (recurrence IN
        ('weekly', 'fortnightly', 'monthly', 'quarterly', 'annually', 'one_shot')),
    first_due TEXT NOT NULL,
    ends_on TEXT CHECK (ends_on IS NULL OR (ends_on >= first_due AND recurrence <> 'one_shot')),
    attention_lead INTEGER CHECK (attention_lead >= 0),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    generate_from TEXT NOT NULL,
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_bill_plans_unit_id ON bill_plans(unit_id);

-- Trigger to update updated_on on every Bill Plan row change
CREATE TRIGGER IF NOT EXISTS trg_bill_plans_set_updated_on
AFTER UPDATE ON bill_plans
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE bill_plans
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;
