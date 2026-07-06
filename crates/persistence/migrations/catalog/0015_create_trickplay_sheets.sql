CREATE TABLE trickplay_sheets (
    version_id TEXT NOT NULL,
    asset_ordinal INTEGER NOT NULL,
    sheet_ordinal INTEGER NOT NULL,
    path TEXT NOT NULL,
    PRIMARY KEY (version_id, asset_ordinal, sheet_ordinal),
    FOREIGN KEY (version_id, asset_ordinal)
        REFERENCES trickplay_assets (version_id, ordinal) ON DELETE CASCADE
);
