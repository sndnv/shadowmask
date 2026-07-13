CREATE TABLE studios (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL
);

CREATE TABLE title_studios (
    title_kind TEXT NOT NULL,
    title_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    studio_id TEXT NOT NULL REFERENCES studios (id) ON DELETE CASCADE,
    PRIMARY KEY (title_kind, title_id, ordinal)
);
