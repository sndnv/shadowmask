CREATE TABLE episodes (
    id TEXT PRIMARY KEY NOT NULL,
    season_id TEXT NOT NULL REFERENCES seasons (id) ON DELETE CASCADE,
    number INTEGER NOT NULL,
    title TEXT NOT NULL,
    overview TEXT,
    runtime_minutes INTEGER,
    air_date INTEGER,
    added_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
