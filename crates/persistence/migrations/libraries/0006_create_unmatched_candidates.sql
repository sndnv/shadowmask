CREATE TABLE unmatched_candidates (
    unmatched_id TEXT NOT NULL REFERENCES unmatched_files (id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL,
    title_kind TEXT NOT NULL,
    title_id TEXT NOT NULL,
    confidence REAL NOT NULL,
    label TEXT NOT NULL,
    PRIMARY KEY (unmatched_id, ordinal)
);
