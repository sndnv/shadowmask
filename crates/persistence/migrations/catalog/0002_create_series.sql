CREATE TABLE series (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    sort_title TEXT NOT NULL,
    year INTEGER,
    overview TEXT,
    rating_system TEXT,
    rating_code TEXT,
    manually_edited INTEGER NOT NULL DEFAULT 0,
    added_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_series_sort_title ON series (sort_title, id);
CREATE INDEX idx_series_added_at ON series (added_at, id);
CREATE INDEX idx_series_year ON series (year, id);
