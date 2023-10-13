CREATE TABLE IF NOT EXISTS signers
(
    signer_id      INTEGER PRIMARY KEY NOT NULL,
    signer_name    TEXT UNIQUE NOT NULL,

    public_key     TEXT UNIQUE NOT NULL,
    secret_key     BLOB NOT NULL,

    created_at     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

INSERT INTO signers (signer_name, public_key, secret_key) VALUES
  ("Alice", "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY", unhex("e5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a")),
  ("Bob", "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty", unhex("398f0c28f98885e046333d4a41c19cee4c37368a9832c6502f6cfd182e2aef89")),
  ("Charlie", "5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y", unhex("bc1ede780f784bb6991a585e4f6e61522c14e1cae6ad0895fb57b9a205a8f938")),
  ("Dave", "5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy", unhex("868020ae0687dda7d57565093a69090211449845a7e11453612800b663307246")),
  ("Eve", "5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw", unhex("786ad0e2df456fe43dd1f91ebca22e235bc162e0bb8d53c633e8c85b2af68b7a")),
  ("Ferdie", "5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSnekmSK2DjL", unhex("42438b7883391c05512a938e36c2df0131e088b3756d6aa7a755fbff19d2f842"))
;
