CREATE TABLE devices (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    platform TEXT NOT NULL,
    last_seen INTEGER
);

CREATE INDEX idx_devices_user ON devices (user_id);
