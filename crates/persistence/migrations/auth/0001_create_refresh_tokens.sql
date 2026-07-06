CREATE TABLE refresh_tokens (
    jti TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    token_hash TEXT NOT NULL,
    issued_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
);

CREATE INDEX idx_refresh_tokens_user ON refresh_tokens (user_id);
