CREATE TABLE audio_tracks (
    version_id TEXT NOT NULL REFERENCES versions (id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL,
    stream_index INTEGER NOT NULL,
    codec TEXT NOT NULL,
    channels INTEGER NOT NULL,
    language TEXT,
    bitrate INTEGER,
    PRIMARY KEY (version_id, ordinal)
);
