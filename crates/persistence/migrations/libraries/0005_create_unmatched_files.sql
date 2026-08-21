CREATE TABLE unmatched_files (
    id TEXT PRIMARY KEY NOT NULL,
    library_id TEXT NOT NULL,
    path TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_unmatched_library ON unmatched_files (library_id, status, id);
