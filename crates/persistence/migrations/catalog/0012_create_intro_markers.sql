CREATE TABLE intro_markers (
    version_id TEXT NOT NULL REFERENCES versions (id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL,
    start_ms INTEGER NOT NULL,
    end_ms INTEGER NOT NULL,
    PRIMARY KEY (version_id, ordinal)
);
