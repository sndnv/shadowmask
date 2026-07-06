CREATE TABLE user_library_access (
    user_id TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    library_id TEXT NOT NULL,
    PRIMARY KEY (user_id, library_id)
);
