CREATE TABLE episodes (
    id TEXT PRIMARY KEY NOT NULL,
    season_id TEXT NOT NULL REFERENCES seasons (id) ON DELETE CASCADE,
    number INTEGER NOT NULL,
    title TEXT NOT NULL,
    overview TEXT,
    runtime_minutes INTEGER,
    air_date INTEGER,
    manually_edited INTEGER NOT NULL DEFAULT 0,
    added_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_episodes_season ON episodes (season_id, number, id);
CREATE INDEX idx_episodes_recent ON episodes (added_at DESC, id ASC);
