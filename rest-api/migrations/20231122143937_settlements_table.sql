CREATE TABLE IF NOT EXISTS settlements
(
    settlement_id   INTEGER PRIMARY KEY NOT NULL,

    venue_id        INTEGER NOT NULL,

    legs            TEXT NOT NULL,

    memo            TEXT,

    created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
