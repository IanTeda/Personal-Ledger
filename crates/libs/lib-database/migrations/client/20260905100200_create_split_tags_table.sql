-- Migration: create split_tags table -- the Tags on a Split, any number of them, in the order the
-- Split lists them (tag_position). Deleting a Split, or its Transaction, deletes its Tags.
--
-- tag_id has no foreign key yet: Tags aren't persisted, so no row would back it (#774). It
-- becomes REFERENCES tags(id) when the Tags domain persists.

CREATE TABLE IF NOT EXISTS split_tags (
    transaction_id UUID NOT NULL,
    position INTEGER NOT NULL,
    tag_position INTEGER NOT NULL CHECK (tag_position >= 0),
    tag_id UUID NOT NULL,
    PRIMARY KEY (transaction_id, position, tag_position),
    FOREIGN KEY (transaction_id, position)
        REFERENCES splits(transaction_id, position) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_split_tags_tag_id ON split_tags(tag_id);
