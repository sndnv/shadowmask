CREATE TABLE versions (
    id TEXT PRIMARY KEY NOT NULL,
    title_kind TEXT NOT NULL,
    title_id TEXT NOT NULL,
    library_id TEXT NOT NULL,
    quality TEXT NOT NULL,
    container TEXT NOT NULL,
    path TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    duration_ms INTEGER NOT NULL,
    available INTEGER NOT NULL DEFAULT 1,
    added_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_versions_title ON versions (title_kind, title_id);
CREATE INDEX idx_versions_library_path ON versions (library_id, path);
