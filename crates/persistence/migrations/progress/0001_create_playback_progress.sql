CREATE TABLE playback_progress (
    version_id    TEXT    NOT NULL PRIMARY KEY,
    position_ms   INTEGER NOT NULL,
    audio_track   INTEGER,
    subtitle_kind TEXT,
    subtitle_ref  TEXT,
    updated_at    INTEGER NOT NULL
);
