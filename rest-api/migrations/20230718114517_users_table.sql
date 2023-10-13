CREATE TABLE IF NOT EXISTS users
(
    user_id        INTEGER PRIMARY KEY NOT NULL,
    username       TEXT UNIQUE NOT NULL,

    created_at     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

INSERT INTO users (username) VALUES("Default");
