CREATE TABLE series (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    year INTEGER,
    overview TEXT,
    rating_system TEXT,
    rating_code TEXT,
    added_at INTEGER NOT NULL
);
