-- Migration: create bill_schedule_entries table -- the dated instances of a Bill Plan
-- (GLOSSARY.md's Bill Schedule), generated ahead of their due dates (ADR-0019). The id is a
-- UUIDv7 like every other row's, and (bill_plan_id, due) is unique: generation looks an entry up
-- by its Plan and due date before adding one, so a date never has two rows (ADR-0019 as amended).
-- An entry is never deleted; one a Bill Plan edit replaces is marked superseded.
--
-- resolution is how the entry was settled. A Paid entry's link is not here but on the Split that
-- pays it (splits.bill_schedule_entry_id), so deleting that Split or its Transaction leaves the
-- entry with no link, which reads as Unmatched. skipped_planned is the Planned Amount at the moment
-- an entry was Skipped, a decimal string, and only a Skipped entry has one.

CREATE TABLE IF NOT EXISTS bill_schedule_entries (
    id UUID PRIMARY KEY,
    bill_plan_id UUID NOT NULL REFERENCES bill_plans(id),
    due TEXT NOT NULL,
    resolution TEXT NOT NULL DEFAULT 'unresolved'
        CHECK (resolution IN ('unresolved', 'paid', 'skipped')),
    skipped_planned TEXT,
    superseded BOOLEAN NOT NULL DEFAULT FALSE,
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    UNIQUE (bill_plan_id, due),
    CHECK ((resolution = 'skipped') = (skipped_planned IS NOT NULL))
);

-- Trigger to update updated_on on every Bill Schedule entry row change
CREATE TRIGGER IF NOT EXISTS trg_bill_schedule_entries_set_updated_on
AFTER UPDATE ON bill_schedule_entries
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE bill_schedule_entries
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;
