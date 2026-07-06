CREATE TABLE subtitle_tracks (
    version_id TEXT NOT NULL REFERENCES versions (id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL,
    stream_index INTEGER NOT NULL,
    language TEXT,
    format TEXT NOT NULL,
    forced INTEGER NOT NULL,
    is_default INTEGER NOT NULL,
    PRIMARY KEY (version_id, ordinal)
);
