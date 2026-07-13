CREATE TABLE title_ratings (
    title_kind TEXT NOT NULL,
    title_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    source TEXT NOT NULL,
    value REAL NOT NULL,
    PRIMARY KEY (title_kind, title_id, ordinal)
);

CREATE TABLE title_external_ids (
    title_kind TEXT NOT NULL,
    title_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    source TEXT NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (title_kind, title_id, ordinal)
);

CREATE TABLE title_extras (
    title_kind TEXT NOT NULL,
    title_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    path TEXT NOT NULL,
    PRIMARY KEY (title_kind, title_id, ordinal)
);
