CREATE TABLE playlist_items (
    playlist_id TEXT    NOT NULL,
    position    INTEGER NOT NULL,
    title_kind  TEXT    NOT NULL,
    title_id    TEXT    NOT NULL,
    added_at    INTEGER NOT NULL,
    PRIMARY KEY (playlist_id, title_kind, title_id)
);

CREATE INDEX idx_playlist_items_position ON playlist_items (playlist_id, position);
CREATE INDEX idx_playlist_items_title ON playlist_items (title_kind, title_id);
