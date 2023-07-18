CREATE TABLE IF NOT EXISTS account_balances
(
    id             INTEGER PRIMARY KEY NOT NULL,
    account_id     INTEGER NOT NULL,
    asset_id       INTEGER NOT NULL,

    balance        INTEGER DEFAULT 0 NOT NULL,
    enc_balance    BLOB NOT NULL,

    created_at     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,

    FOREIGN KEY(account_id) REFERENCES accounts(id),
    FOREIGN KEY(asset_id) REFERENCES assets(id),
		UNIQUE (account_id, asset_id)
);
