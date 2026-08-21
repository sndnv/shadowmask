CREATE TABLE movies (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    sort_title TEXT NOT NULL,
    year INTEGER,
    overview TEXT,
    runtime_minutes INTEGER,
    rating_system TEXT,
    rating_code TEXT,
    manually_edited INTEGER NOT NULL DEFAULT 0,
    added_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_movies_sort_title ON movies (sort_title, id);
CREATE INDEX idx_movies_added_at ON movies (added_at, id);
CREATE INDEX idx_movies_year ON movies (year, id);
