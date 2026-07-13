CREATE TABLE people (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL
);

CREATE TABLE credits (
    title_kind TEXT NOT NULL,
    title_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    person_id TEXT NOT NULL REFERENCES people (id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    character TEXT,
    credit_order INTEGER NOT NULL,
    PRIMARY KEY (title_kind, title_id, ordinal)
);

CREATE INDEX idx_credits_person ON credits (person_id);
