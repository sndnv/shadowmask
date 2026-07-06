CREATE TABLE user_preferred_subtitle (
    user_id TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL,
    lang TEXT NOT NULL,
    PRIMARY KEY (user_id, ordinal)
);
