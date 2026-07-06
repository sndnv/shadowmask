CREATE TABLE watchlist (
    title_kind TEXT    NOT NULL,
    title_id   TEXT    NOT NULL,
    added_at   INTEGER NOT NULL,
    PRIMARY KEY (title_kind, title_id)
);
