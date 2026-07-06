CREATE TABLE library_roots (
    library_id TEXT NOT NULL REFERENCES libraries (id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL,
    path TEXT NOT NULL,
    PRIMARY KEY (library_id, ordinal)
);
