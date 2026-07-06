CREATE TABLE link_codes (
    code TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL,
    expires_at INTEGER NOT NULL
);
