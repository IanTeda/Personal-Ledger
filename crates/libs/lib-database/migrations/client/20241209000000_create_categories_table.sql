-- Migration: create categories table -- the tree a Ledger's Splits and Bill Plans are filed under
-- (GLOSSARY.md's Category, #858). Created first, since `payees`, `bill_plans`, `splits` and
-- `budgets` reference it.
--
-- parent_id is NULL for a top-level Category and otherwise references the parent, with no cascade:
-- only a leaf is deleted, and the key is the backstop to that rule. The three-level cap, a child
-- taking its parent's type and the refusal of a cycle are lib-categories' rules, not constraints.
-- name isn't unique: each type has its own top-level Uncategorised, and siblings are a rule's to
-- tell apart. Rows read back in rowid order, the order they were added in, which an upsert keeps,
-- so siblings list in that order under their parent.

CREATE TABLE IF NOT EXISTS categories (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    parent_id UUID REFERENCES categories(id),
    category_type TEXT NOT NULL CHECK (category_type IN ('expense', 'income')),
    created_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_categories_parent_id ON categories(parent_id);

CREATE TRIGGER IF NOT EXISTS trg_categories_set_updated_on
AFTER UPDATE ON categories
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE categories
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;
