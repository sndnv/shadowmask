CREATE TABLE queue_items (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    position   INTEGER NOT NULL,
    title_kind TEXT    NOT NULL,
    title_id   TEXT    NOT NULL
);

CREATE INDEX idx_queue_items_position ON queue_items (position, id);
CREATE INDEX idx_queue_items_title ON queue_items (title_kind, title_id, position);
