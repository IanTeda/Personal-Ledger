-- Migration: create price_sources table -- the stored Price Source rows (ADR-0043): a Custom
-- source a person added, with its address, request, price path, fixed request headers and the
-- name its API key travels under, or a person's own name and update interval for a built-in. A built-in's
-- definition and Manual entry are code in lib-units, so a built-in has a row only once it is
-- renamed or retimed, and Manual entry never has one. That is why units.price_source_id has no
-- foreign key here.
--
-- No column holds an API key: a key is in price_source_keys, encrypted (#1391), never here. Only
-- key_placement and key_name, which say where a key goes in the request, are stored. A Custom source declares no Unit Kinds (lib-units offers it for
-- every Kind), so there is no column for them.

CREATE TABLE IF NOT EXISTS price_sources (
    id UUID PRIMARY KEY,
    origin TEXT NOT NULL CHECK (origin IN ('built_in', 'custom')),
    name TEXT NOT NULL,
    updates TEXT NOT NULL CHECK (updates IN
        ('every_15_minutes', 'hourly', 'daily', 'manual_only')),
    address TEXT,
    request TEXT,
    price_path TEXT,
    key_placement TEXT CHECK (key_placement IN ('param', 'header')),
    key_name TEXT,
    -- A Custom source's fixed request headers, never secret: one `Name: value` per line, in the
    -- order sent, or NULL for none. A header can't hold a line break, so a line is a header.
    headers TEXT,
    -- A Custom source's optional history request (ADR-0043's amendment "a Custom source's
    -- history request"): its request, the JSON pointers to the series and, relative to each
    -- point, its date and price, and the series' shape. All NULL for none; a built-in's is code.
    history_request TEXT,
    history_series_path TEXT,
    history_date_path TEXT,
    history_price_path TEXT,
    history_shape TEXT CHECK (history_shape IN ('points', 'keyed')),
    created_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    -- A Custom source has its whole endpoint; a built-in's endpoint is code.
    CHECK ((origin = 'custom') =
        (address IS NOT NULL AND request IS NOT NULL AND price_path IS NOT NULL)),
    -- A key's placement and name come together, and only on a Custom source.
    CHECK ((key_placement IS NULL) = (key_name IS NULL)),
    CHECK (origin = 'custom' OR key_placement IS NULL),
    -- A built-in's headers are code.
    CHECK (origin = 'custom' OR headers IS NULL),
    -- A history request comes whole, and only on a Custom source.
    CHECK ((history_request IS NULL) = (history_series_path IS NULL)
        AND (history_request IS NULL) = (history_date_path IS NULL)
        AND (history_request IS NULL) = (history_price_path IS NULL)
        AND (history_request IS NULL) = (history_shape IS NULL)),
    CHECK (origin = 'custom' OR history_request IS NULL)
);


-- Trigger to update updated_on on every price source row change
CREATE TRIGGER IF NOT EXISTS trg_price_sources_set_updated_on
AFTER UPDATE ON price_sources
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE price_sources
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;
