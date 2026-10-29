-- Migration: create unit_prices table -- a Unit's Weekly Prices (GLOSSARY.md's Weekly Price,
-- ADR-0052): one row per Unit per ISO week, keyed by that week's Monday, stated in the Unit's
-- quote Unit and kept forever. close is required; open, high and low are optional, since Manual
-- entry usually gives a close only. Prices are decimal strings so no precision is lost to a
-- float. written_on is when the row was last written, the last price's age. Deleting a Unit
-- deletes its Weekly Prices.

CREATE TABLE IF NOT EXISTS unit_prices (
    unit_id UUID NOT NULL REFERENCES units(id) ON DELETE CASCADE,
    week TEXT NOT NULL,
    close TEXT NOT NULL,
    open TEXT,
    high TEXT,
    low TEXT,
    written_on TEXT NOT NULL,
    PRIMARY KEY (unit_id, week)
);
