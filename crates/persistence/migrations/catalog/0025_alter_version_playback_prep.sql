DELETE FROM version_playback_prep;

ALTER TABLE version_playback_prep RENAME COLUMN keyframes TO keyframes_us;

ALTER TABLE version_playback_prep ADD COLUMN prepared_with TEXT NOT NULL DEFAULT '';
