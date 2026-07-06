CREATE TABLE scan_state (
    library_id TEXT PRIMARY KEY NOT NULL,
    status TEXT NOT NULL,
    progress REAL NOT NULL,
    last_scanned_at INTEGER,
    error TEXT
);
