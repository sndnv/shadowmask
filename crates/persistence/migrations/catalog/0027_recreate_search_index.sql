DROP TABLE search_index;

CREATE VIRTUAL TABLE search_index USING fts5 (
    kind,
    id UNINDEXED,
    title,
    terms,
    tokenize = 'unicode61 remove_diacritics 0'
);

CREATE VIRTUAL TABLE search_vocabulary USING fts5vocab (search_index, row);
