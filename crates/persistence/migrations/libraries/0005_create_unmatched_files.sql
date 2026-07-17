CREATE TABLE unmatched_files (
    id TEXT PRIMARY KEY NOT NULL,
    library_id TEXT NOT NULL,
    path TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
