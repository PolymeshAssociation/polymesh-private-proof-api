-- DROP column ticker from assets table.
PRAGMA foreign_keys = OFF;

CREATE TABLE IF NOT EXISTS assets_new
(
    asset_id       TEXT PRIMARY KEY NOT NULL,

    created_at     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

INSERT INTO assets_new SELECT asset_id, created_at, updated_at FROM assets;

DROP TABLE assets;

ALTER TABLE assets_new RENAME TO assets;

PRAGMA foreign_keys = ON;
