CREATE TABLE IF NOT EXISTS settlement_events
(
    id              INTEGER PRIMARY KEY NOT NULL,

    settlement_id   INTEGER NOT NULL,

    event           TEXT NOT NULL,

    created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,

    FOREIGN KEY(settlement_id) REFERENCES settlements(settlement_id)
);
