CREATE TABLE IF NOT EXISTS transactions
(
    id     INTEGER PRIMARY KEY NOT NULL,

    block_hash    TEXT NOT NULL,
    block_number  INTEGER NOT NULL,
    tx_hash       TEXT NOT NULL,

    success       INTEGER NOT NULL,
    error         TEXT,

    events        TEXT,

    created_at     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,

		UNIQUE (block_hash, tx_hash)
);
