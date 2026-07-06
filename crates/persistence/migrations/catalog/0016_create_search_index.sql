CREATE VIRTUAL TABLE search_index USING fts5 (
    kind UNINDEXED,
    id UNINDEXED,
    title
);
