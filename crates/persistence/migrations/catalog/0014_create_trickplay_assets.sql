CREATE TABLE trickplay_assets (
    version_id TEXT NOT NULL REFERENCES versions (id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL,
    interval_ms INTEGER NOT NULL,
    tile_width INTEGER NOT NULL,
    tile_height INTEGER NOT NULL,
    grid_columns INTEGER NOT NULL DEFAULT 0,
    grid_rows INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (version_id, ordinal)
);
