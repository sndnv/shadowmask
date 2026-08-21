CREATE TABLE subtitle_files (
    id TEXT PRIMARY KEY,
    version_id TEXT NOT NULL REFERENCES versions (id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL,
    language TEXT,
    format TEXT NOT NULL,
    source TEXT NOT NULL,
    path TEXT NOT NULL,
    translated_from TEXT,
    label TEXT,
    pinned INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_subtitle_files_version ON subtitle_files (version_id);
