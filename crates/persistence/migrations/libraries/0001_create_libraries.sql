CREATE TABLE libraries (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    origin TEXT NOT NULL DEFAULT 'local',
    watcher TEXT NOT NULL,
    scan_schedule TEXT,
    sort_articles TEXT NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
