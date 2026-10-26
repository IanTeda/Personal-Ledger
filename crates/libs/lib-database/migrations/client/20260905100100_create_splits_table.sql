-- Migration: create splits table -- the lines of a Transaction (GLOSSARY.md's Split): an amount
-- and a leaf Category, with an optional Payee (ADR-0012). Keyed by its Transaction and its
-- position, the order a person entered them, so a Transaction's Splits read back in that order.
-- Deleting a Transaction deletes its Splits. amount is a decimal string so no precision is lost
-- to a float; expenses are negative and income positive.
--
-- bill_schedule_entry_id is the Bill Schedule entry this Split pays, if any: the Match (ADR-0019,
-- settlement by Match, at the Split). It is unique, so a Split pays at most one entry and an entry
-- is paid by at most one Split.
--
-- payee_id references the Payee, with no cascade: a Payee a Split names can only be deactivated
-- (ADR-0012), and the key is the backstop to that rule. category_id references the Category, with
-- no cascade: Category delete re-points a Split to Uncategorised first, and the key backstops that
-- (#858).

CREATE TABLE IF NOT EXISTS splits (
    transaction_id UUID NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    amount TEXT NOT NULL,
    category_id UUID NOT NULL REFERENCES categories(id),
    payee_id UUID REFERENCES payees(id),
    bill_schedule_entry_id UUID UNIQUE REFERENCES bill_schedule_entries(id),
    PRIMARY KEY (transaction_id, position)
);

CREATE INDEX IF NOT EXISTS idx_splits_category_id ON splits(category_id);
CREATE INDEX IF NOT EXISTS idx_splits_payee_id ON splits(payee_id);
