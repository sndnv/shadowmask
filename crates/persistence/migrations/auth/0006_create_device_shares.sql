CREATE TABLE device_shares (
    device_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (device_id, user_id)
);

CREATE INDEX idx_device_shares_user ON device_shares (user_id);
