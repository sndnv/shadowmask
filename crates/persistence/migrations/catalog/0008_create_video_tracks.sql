CREATE TABLE video_tracks (
    version_id TEXT NOT NULL REFERENCES versions (id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL,
    stream_index INTEGER NOT NULL,
    codec TEXT NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    bit_depth INTEGER NOT NULL,
    hdr TEXT,
    frame_rate REAL NOT NULL,
    bitrate INTEGER,
    PRIMARY KEY (version_id, ordinal)
);
