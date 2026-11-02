-- Migration: create document_types table -- the Ledger's user-managed Document Types (GLOSSARY.md's
-- Document Type, ADR-0031), in the user's order (#963).
--
-- position is the user's order, rewritten whole whenever a type moves or goes, so it is never
-- unique while a batch renumbers. A name is unique ignoring case. tracks_date is the Key Date kind
-- the type tracks, and remind_unit and remind_count its Remind lead time, set together and inert
-- while tracks_date is null. files is the type's Filed count as the stub seeds it, kept as it is
-- until it is counted from the Documents. One type is the Default (Other), which the partial unique
-- index holds; it is never removed.

CREATE TABLE IF NOT EXISTS document_types (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL COLLATE NOCASE UNIQUE,
    position INTEGER NOT NULL,
    tracks_date TEXT CHECK (tracks_date IN ('renews', 'ends', 'expires', 'revalue')),
    remind_unit TEXT CHECK (remind_unit IN ('days', 'months')),
    remind_count INTEGER CHECK (remind_count >= 0),
    financial_year BOOLEAN NOT NULL DEFAULT FALSE,
    files INTEGER NOT NULL DEFAULT 0 CHECK (files >= 0),
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    CHECK ((remind_unit IS NULL) = (remind_count IS NULL))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_document_types_one_default
    ON document_types(is_default) WHERE is_default;

CREATE TRIGGER IF NOT EXISTS trg_document_types_set_updated_on
AFTER UPDATE ON document_types
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE document_types
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;
