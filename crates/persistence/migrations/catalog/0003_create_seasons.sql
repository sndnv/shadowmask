CREATE TABLE seasons (
    id TEXT PRIMARY KEY NOT NULL,
    series_id TEXT NOT NULL REFERENCES series (id) ON DELETE CASCADE,
    number INTEGER NOT NULL,
    title TEXT,
    overview TEXT,
    added_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_seasons_series ON seasons (series_id, number, id);
