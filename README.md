[![Discord](https://img.shields.io/badge/Discord-Join_our_server-blue.svg?style=social&logo=discord)](https://discord.com/invite/ud2deWAnyt) 
![Twitter Follow](https://img.shields.io/twitter/follow/PolymeshNetwork?style=social)

<img src="Polymesh-logo.svg" width="70%" alt="Polymesh"/>

# Confidential Asset Server

This repository provides functionality to interact with Confidential Assets on Polymesh.

Functionality is split into:
  - Proof API: <https://github.com/PolymeshAssociation/confidential_assets_server/tree/main/proof-api>
  - REST API: <https://github.com/PolymeshAssociation/confidential_assets_server/tree/main/rest-api>

# Proof API

The Proof API provides endpoints to allow users to:
  - Generate and Store Confidential Accounts: A confidential account is an Elgamal Key pair used to store encrypted balances and generate proofs with respect to those balances.
  - Generate Sender Proofs: Proofs are required when transacting confidential assets on Polymesh - these proofs establish the validity of transactions, without revealing their underlying balances.
  - Verify Proofs: Mediators and asset receivers need to verify sender proofs to check that they are referencing expected amounts and other details.
  - Decrypt Amounts: Investors can decrypt their encrypted on-chain balances, using their stored confidential accounts.

## Build and Run

The Proof API can be built via:
```bash
cd proof-api
cargo build --release
```

Once built, you must set the `DATABASE_URL` environment variable as per <https://github.com/PolymeshAssociation/confidential_assets_server/blob/main/proof-api/.env.example>.

You can the initialise the database via:
```bash
cargo install sqlx-cli
sqlx database setup
```

Once the database is initialised, you can run the Proof API via:
```bash
cargo run --release
```

# REST API

The REST API will directly interact with a Polymesh node to submit Confidential Asset transactions on-chain, and read relevant chain storage.

This part of the project is expected to be deprecated in favour of <https://github.com/PolymeshAssociation/polymesh-rest-api>.
