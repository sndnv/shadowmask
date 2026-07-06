CREATE TABLE libraries (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    watcher TEXT NOT NULL,
    scan_schedule TEXT
);
