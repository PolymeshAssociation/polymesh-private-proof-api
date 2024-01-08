CREATE TABLE IF NOT EXISTS account_assets
(
    account_asset_id  INTEGER PRIMARY KEY NOT NULL,
    account_id     INTEGER NOT NULL,
    asset_id       TEXT NOT NULL,

    balance        INTEGER DEFAULT 0 NOT NULL,
    enc_balance    BLOB NOT NULL,

    created_at     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,

    FOREIGN KEY(account_id) REFERENCES accounts(account_id),
    FOREIGN KEY(asset_id) REFERENCES assets(asset_id),
		UNIQUE (account_id, asset_id)
);
