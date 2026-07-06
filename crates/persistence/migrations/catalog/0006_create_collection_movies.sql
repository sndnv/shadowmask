CREATE TABLE collection_movies (
    collection_id TEXT NOT NULL REFERENCES collections (id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL,
    movie_id TEXT NOT NULL,
    PRIMARY KEY (collection_id, ordinal)
);
