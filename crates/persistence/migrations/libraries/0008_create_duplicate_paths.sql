CREATE TABLE duplicate_paths (
    duplicate_id TEXT NOT NULL REFERENCES duplicate_candidates (id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL,
    path TEXT NOT NULL,
    PRIMARY KEY (duplicate_id, ordinal)
);
