CREATE TABLE movies (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    year INTEGER,
    overview TEXT,
    runtime_minutes INTEGER,
    rating_system TEXT,
    rating_code TEXT,
    added_at INTEGER NOT NULL
);
