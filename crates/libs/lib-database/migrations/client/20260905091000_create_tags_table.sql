-- Migration: create tags table -- the free-form labels a Split carries (GLOSSARY.md's Tag, #862).
-- Created before `split_tags`, which references `tags(id)`.
--
-- name isn't unique: the normalised-name rule (case, spaces and punctuation ignored) is
-- lib-tags' to apply, and the stub seed holds `work-trip` and `Work Trip` on purpose, a likely
-- duplicate for the merge to show. color is the canonical `#RRGGBB` or NULL for no swatch. Rows
-- read back in rowid order, the order they were added in, which an upsert keeps.

CREATE TABLE IF NOT EXISTS tags (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    color TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TRIGGER IF NOT EXISTS trg_tags_set_updated_on
AFTER UPDATE ON tags
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE tags
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;
