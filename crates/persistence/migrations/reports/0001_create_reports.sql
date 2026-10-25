CREATE TABLE reports (
    id TEXT PRIMARY KEY NOT NULL,
    created_at INTEGER NOT NULL,
    user_id TEXT NOT NULL,
    player_id TEXT NOT NULL,
    app TEXT NOT NULL,
    platform TEXT NOT NULL,
    app_version TEXT NOT NULL,
    note TEXT,
    payload TEXT NOT NULL
);

CREATE INDEX reports_created_idx ON reports (created_at DESC, id);
