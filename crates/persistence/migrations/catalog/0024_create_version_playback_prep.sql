CREATE TABLE version_playback_prep (
    version_id TEXT PRIMARY KEY REFERENCES versions (id) ON DELETE CASCADE,
    keyframes TEXT NOT NULL,
    extracted_subtitles TEXT NOT NULL
);
