-- DROP column ticker from assets table.
PRAGMA foreign_keys = OFF;

CREATE TABLE assets_new
(
    asset_id       TEXT PRIMARY KEY NOT NULL,

    created_at     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

INSERT INTO assets_new SELECT asset_id, created_at, updated_at FROM assets;

CREATE TABLE account_assets_new
(
    account_asset_id  INTEGER PRIMARY KEY NOT NULL,
    account_id     INTEGER NOT NULL,
    asset_id       TEXT NOT NULL,

    balance        INTEGER DEFAULT 0 NOT NULL,
    enc_balance    BLOB NOT NULL,

    created_at     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,

    FOREIGN KEY(account_id) REFERENCES accounts(account_id),
    FOREIGN KEY(asset_id) REFERENCES assets_new(asset_id),
		UNIQUE (account_id, asset_id)
);
INSERT INTO account_assets_new SELECT * FROM account_assets;

DROP TABLE account_assets;
DROP TABLE assets;

PRAGMA foreign_keys = ON;

ALTER TABLE account_assets_new RENAME TO account_assets;
ALTER TABLE assets_new RENAME TO assets;

