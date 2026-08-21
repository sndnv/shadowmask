CREATE TABLE duplicate_candidates (
    id TEXT PRIMARY KEY NOT NULL,
    library_id TEXT NOT NULL,
    title_kind TEXT NOT NULL,
    title_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active'
);

CREATE INDEX idx_duplicates_library ON duplicate_candidates (library_id, status, id);
