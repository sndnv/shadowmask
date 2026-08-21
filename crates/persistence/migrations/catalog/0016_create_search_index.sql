CREATE VIRTUAL TABLE search_index USING fts5 (
    kind,
    id UNINDEXED,
    title
);
