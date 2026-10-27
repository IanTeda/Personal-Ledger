-- Migration: create locations table -- a place whose contents are kept as one register
-- (GLOSSARY.md's Location, #866). Created before `areas` and `items`, which reference it.
--
-- unit_id is the fiat Unit the Location's values are in, fixed when it is added, and references
-- the Unit with no cascade. The insurance cover is one of the Location's details, not a record of
-- its own, so its five columns sit here and are all NULL when there is no cover: cover_insurer is
-- the switch, and cover_sum_insured is set whenever it is. The amounts are decimal strings in the
-- Unit, so no precision is lost to a float; cover_renews_on is a calendar date. Name uniqueness,
-- ignoring case, is lib-inventory's rule, so the column carries none. Rows read back in rowid
-- order, the order they were added in, which an upsert keeps.

CREATE TABLE IF NOT EXISTS locations (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    address TEXT NOT NULL,
    unit_id UUID NOT NULL REFERENCES units(id),
    cover_insurer TEXT,
    cover_policy_no TEXT,
    cover_renews_on TEXT,
    cover_sum_insured TEXT,
    cover_item_limit TEXT,
    created_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    CHECK ((cover_insurer IS NULL) = (cover_sum_insured IS NULL)),
    CHECK (cover_insurer IS NOT NULL OR
        (cover_policy_no IS NULL AND cover_renews_on IS NULL AND cover_item_limit IS NULL))
);

CREATE INDEX IF NOT EXISTS idx_locations_unit_id ON locations(unit_id);

CREATE TRIGGER IF NOT EXISTS trg_locations_set_updated_on
AFTER UPDATE ON locations
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE locations
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;
