CREATE TABLE collection_movies (
    collection_id TEXT NOT NULL REFERENCES collections (id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL,
    movie_id TEXT NOT NULL,
    PRIMARY KEY (collection_id, ordinal)
);

CREATE INDEX idx_collection_movies_movie ON collection_movies (movie_id);
