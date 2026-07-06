CREATE TABLE subtitle_offsets (
    version_id    TEXT    NOT NULL,
    subtitle_kind TEXT    NOT NULL,
    subtitle_ref  TEXT    NOT NULL,
    offset_ms     INTEGER NOT NULL,
    PRIMARY KEY (version_id, subtitle_kind, subtitle_ref)
);
