CREATE TABLE artwork_widths (
    artwork_id TEXT NOT NULL REFERENCES artwork (artwork_id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL,
    width INTEGER NOT NULL,
    PRIMARY KEY (artwork_id, ordinal)
);
