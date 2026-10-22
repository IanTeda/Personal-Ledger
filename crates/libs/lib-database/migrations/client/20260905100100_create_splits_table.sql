-- Migration: create splits table -- the lines of a Transaction (GLOSSARY.md's Split): an amount
-- and a leaf Category, with an optional Payee (ADR-0012). Keyed by its Transaction and its
-- position, the order a person entered them, so a Transaction's Splits read back in that order.
-- Deleting a Transaction deletes its Splits. amount is a decimal string so no precision is lost
-- to a float; expenses are negative and income positive.
--
-- category_id and payee_id have no foreign keys yet: Categories and Payees aren't persisted, so
-- no row would back them (#774). Each becomes a REFERENCES when its domain persists.

CREATE TABLE IF NOT EXISTS splits (
    transaction_id UUID NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    amount TEXT NOT NULL,
    category_id UUID NOT NULL,
    payee_id UUID,
    PRIMARY KEY (transaction_id, position)
);

CREATE INDEX IF NOT EXISTS idx_splits_category_id ON splits(category_id);
CREATE INDEX IF NOT EXISTS idx_splits_payee_id ON splits(payee_id);
