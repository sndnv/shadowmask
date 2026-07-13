CREATE TABLE genres (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL
);

CREATE TABLE title_genres (
    title_kind TEXT NOT NULL,
    title_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    genre_id TEXT NOT NULL REFERENCES genres (id) ON DELETE CASCADE,
    PRIMARY KEY (title_kind, title_id, ordinal)
);

CREATE INDEX idx_title_genres_genre ON title_genres (genre_id);
