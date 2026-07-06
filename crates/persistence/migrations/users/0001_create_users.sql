CREATE TABLE users (
    id TEXT PRIMARY KEY NOT NULL,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL,
    max_rating_system TEXT,
    max_rating_code TEXT,
    concurrent_stream_limit INTEGER,
    bitrate_cap INTEGER,
    created_at INTEGER NOT NULL
);
