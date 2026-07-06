CREATE TABLE playback_progress (
    version_id  TEXT    NOT NULL PRIMARY KEY,
    position_ms INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);
