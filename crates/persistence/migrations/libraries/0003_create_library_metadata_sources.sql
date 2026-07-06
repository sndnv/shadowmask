CREATE TABLE library_metadata_sources (
    library_id TEXT NOT NULL REFERENCES libraries (id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL,
    source TEXT NOT NULL,
    PRIMARY KEY (library_id, ordinal)
);
