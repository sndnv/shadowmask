CREATE TABLE seasons (
    id TEXT PRIMARY KEY NOT NULL,
    series_id TEXT NOT NULL REFERENCES series (id) ON DELETE CASCADE,
    number INTEGER NOT NULL,
    title TEXT,
    overview TEXT
);
