CREATE TABLE IF NOT EXISTS assets
(
    asset_id       TEXT PRIMARY KEY NOT NULL,
    ticker         TEXT UNIQUE,

    created_at     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
