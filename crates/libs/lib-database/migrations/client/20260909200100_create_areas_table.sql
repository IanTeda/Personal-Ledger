-- Migration: create areas table -- the named areas of one Location (GLOSSARY.md's Area, #866).
-- Created after `locations` and before `items`, which reference it.
--
-- position is the Area's place in its Location's tabs, from 0; the adapter rewrites a Location's
-- Areas with it whenever the Location is written, so it isn't unique mid-write. An Area never
-- changes Location. location_id references the Location with no cascade: removing a Location
-- removes its Items and Areas first, and the key is the backstop to that. Name uniqueness within
-- a Location, ignoring case, is lib-inventory's rule. (id, location_id) is unique so an Item can
-- name its Area and Location together.

CREATE TABLE IF NOT EXISTS areas (
    id UUID PRIMARY KEY,
    location_id UUID NOT NULL REFERENCES locations(id),
    name TEXT NOT NULL,
    position INTEGER NOT NULL CHECK (position >= 0),
    created_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    UNIQUE (id, location_id)
);

CREATE INDEX IF NOT EXISTS idx_areas_location_id ON areas(location_id, position);

CREATE TRIGGER IF NOT EXISTS trg_areas_set_updated_on
AFTER UPDATE ON areas
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE areas
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;
