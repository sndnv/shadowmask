CREATE TABLE watch_history (
    title_kind      TEXT    NOT NULL,
    title_id        TEXT    NOT NULL,
    watched         INTEGER NOT NULL,
    play_count      INTEGER NOT NULL,
    last_watched_at INTEGER,
    completed       INTEGER NOT NULL,
    PRIMARY KEY (title_kind, title_id)
);
