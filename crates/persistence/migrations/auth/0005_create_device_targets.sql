CREATE TABLE device_targets (
    device_id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,
    addresses TEXT NOT NULL,
    port INTEGER NOT NULL,
    app_id TEXT,
    reported_at INTEGER NOT NULL
);
