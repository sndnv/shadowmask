CREATE TABLE chapters (
    version_id TEXT NOT NULL REFERENCES versions (id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL,
    title TEXT NOT NULL,
    start_ms INTEGER NOT NULL,
    PRIMARY KEY (version_id, ordinal)
);
