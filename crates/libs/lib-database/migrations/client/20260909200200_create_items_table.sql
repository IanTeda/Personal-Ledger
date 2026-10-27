-- Migration: create items table -- the physical possessions in one Area of one Location
-- (GLOSSARY.md's Inventory Item, #866).
--
-- An Item references its Location and its Area, and the pair together, so it can't name an Area
-- of another Location; none cascades. Remove Area moves the Items to another Area first, and
-- Remove Location removes them first, and the keys are the backstop to those rules.
-- replacement is a decimal string in the Location's Unit, so no precision is lost to a float.
-- Rows read back in rowid order, the order they were added in, which an upsert keeps.

CREATE TABLE IF NOT EXISTS items (
    id UUID PRIMARY KEY,
    location_id UUID NOT NULL REFERENCES locations(id),
    area_id UUID NOT NULL REFERENCES areas(id),
    name TEXT NOT NULL,
    replacement TEXT NOT NULL,
    created_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    FOREIGN KEY (area_id, location_id) REFERENCES areas(id, location_id)
);

CREATE INDEX IF NOT EXISTS idx_items_location_id ON items(location_id);
CREATE INDEX IF NOT EXISTS idx_items_area_id ON items(area_id, location_id);

CREATE TRIGGER IF NOT EXISTS trg_items_set_updated_on
AFTER UPDATE ON items
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE items
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;
