-- Migration: create bill_plans table -- the recurring definition of an expected payment
-- obligation (GLOSSARY.md's Bill Plan, docs/bills.md). planned_amount is a positive decimal
-- string so no precision is lost to a float. A Bill Plan is never deleted, only deactivated, so
-- every Bill Schedule entry keeps its Plan. generate_from is the earliest due date generation may
-- create, so an edit or a reactivation never backfills due dates already past (ADR-0019).
--
-- unit_id, account_id, payee_id and category_id are the foreign keys, the last three with no
-- cascade: a Payee a Bill Plan names can only be deactivated (ADR-0012), and the key is the
-- backstop to that rule; an Account a Bill Plan names can't be deleted out from under it (#853);
-- Category delete re-points a Bill Plan to Uncategorised first, and the key backstops that (#858).

CREATE TABLE IF NOT EXISTS bill_plans (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    category_id UUID NOT NULL REFERENCES categories(id),
    unit_id UUID NOT NULL REFERENCES units(id),
    account_id UUID NOT NULL REFERENCES accounts(id),
    payee_id UUID REFERENCES payees(id),
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
CREATE INDEX IF NOT EXISTS idx_bill_plans_account_id ON bill_plans(account_id);
CREATE INDEX IF NOT EXISTS idx_bill_plans_payee_id ON bill_plans(payee_id);
CREATE INDEX IF NOT EXISTS idx_bill_plans_category_id ON bill_plans(category_id);

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
