CREATE TABLE jobs (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    priority INTEGER NOT NULL,
    payload TEXT NOT NULL,
    attempts INTEGER NOT NULL,
    progress REAL NOT NULL,
    available_at INTEGER NOT NULL,
    last_error TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    started_at INTEGER,
    finished_at INTEGER,
    parent_id TEXT
);

CREATE INDEX jobs_parent_idx ON jobs (parent_id);
CREATE INDEX jobs_created_idx ON jobs (created_at DESC, id);
CREATE INDEX jobs_ready_idx ON jobs (status, priority DESC, created_at, id);
