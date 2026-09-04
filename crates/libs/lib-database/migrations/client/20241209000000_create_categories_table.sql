-- Migration: create categories table and supporting constraints

CREATE TABLE IF NOT EXISTS categories (
    id UUID PRIMARY KEY,
    code TEXT UNIQUE NOT NULL,
    name TEXT UNIQUE NOT NULL,
    description TEXT,
    url_slug TEXT UNIQUE,
    category_type TEXT NOT NULL CHECK (category_type IN ('asset', 'equity', 'expense', 'income', 'liability')),
    color TEXT CHECK (color IS NULL OR (length(color) = 7 AND substr(color,1,1) = '#')),
    icon TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);


-- Cover frequent look ups by type/active flag and ordering by creation/update timestamps
CREATE INDEX IF NOT EXISTS idx_categories_type_active ON categories(category_type, is_active);
CREATE INDEX IF NOT EXISTS idx_categories_created_on ON categories(created_on DESC);
CREATE INDEX IF NOT EXISTS idx_categories_updated_on ON categories(updated_on DESC);
CREATE INDEX IF NOT EXISTS idx_categories_code ON categories(code);
CREATE INDEX IF NOT EXISTS idx_categories_active_type ON categories(is_active, category_type);


-- Trigger to modify the the updated_on row after update to categories row
CREATE TRIGGER IF NOT EXISTS trg_categories_set_updated_on
AFTER UPDATE ON categories
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE categories
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;