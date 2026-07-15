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
    edition TEXT,
    available INTEGER NOT NULL DEFAULT 1
);
