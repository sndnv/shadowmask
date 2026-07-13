CREATE TABLE api_tokens (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL
);

CREATE INDEX idx_api_tokens_user ON api_tokens (user_id);
