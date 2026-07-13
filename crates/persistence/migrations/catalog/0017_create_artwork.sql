CREATE TABLE artwork (
    artwork_id TEXT PRIMARY KEY NOT NULL,
    title_kind TEXT NOT NULL,
    title_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    kind TEXT NOT NULL
);

CREATE INDEX idx_artwork_owner ON artwork (title_kind, title_id);
