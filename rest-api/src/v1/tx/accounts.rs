use actix_web::{get, post, rt::pin, web, HttpResponse, Responder, Result};
use futures_util::StreamExt;
use uuid::Uuid;

use polymesh_api::types::{
  confidential_assets::transaction::ConfidentialTransferProof as SenderProof,
  pallet_confidential_asset::{
    AffirmLeg, AffirmParty, AffirmTransaction, AffirmTransactions, ConfidentialTransfers,
  },
};
use polymesh_api::Api;

use polymesh-private-proof-api::repo::Repository;
use polymesh-private-proof-shared::{
  auditor_account_to_key, confidential_account_to_key, error::Error, scale_convert,
  AccountAssetIncomingBalance, AffirmTransactionLegRequest, AffirmTransactionsRequest, PublicKey,
  TransactionArgs, TransactionParty, TransactionResult,
};

use super::account_assets;
use crate::signing::AppSigningManager;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg
    .service(tx_init_account)
    .service(tx_account_did)
    .service(tx_apply_incoming_balances)
    .service(get_incoming_balances)
    .service(tx_affirm_transactions)
    .service(tx_mediator_affirm_leg)
    .configure(account_assets::service);
}

/// Add the account on-chain.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/accounts/{public_key}/init_account")]
pub async fn tx_init_account(
  path: web::Path<String>,
  req: web::Json<TransactionArgs>,
  repo: Repository,
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let public_key = path.into_inner();
  let mut signer = signing
    .get_signer(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))?;
  // Get the account.
  let account = repo
    .get_account_with_secret(&public_key)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?;
  let confidential_account = account.as_confidential_account()?;

  let res = api
    .call()
    .confidential_asset()
    .create_account(confidential_account)
    .map_err(|err| Error::from(err))?
    .submit_and_watch(&mut signer)
    .await
    .map_err(|err| Error::from(err))?;

  // Wait for transaction results.
  let res = TransactionResult::wait_for_results(res, req.finalize).await?;
  Ok(HttpResponse::Ok().json(res))
}

/// Get the account's on-chain identity.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/accounts/{public_key}/identity")]
pub async fn tx_account_did(
  path: web::Path<PublicKey>,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let public_key = path.into_inner();
  let confidential_account = public_key.as_confidential_account()?;

  let account_did = api
    .query()
    .confidential_asset()
    .account_did(confidential_account)
    .await
    .map_err(|err| Error::from(err))?
    .ok_or_else(|| Error::not_found("Confidential account doesn't exist"))?;

  Ok(HttpResponse::Ok().json(account_did))
}

/// Query chain for an account's incoming balances.
#[utoipa::path(
  responses(
    (status = 200, body = Vec<AccountAssetIncomingBalance>)
  )
)]
#[get("/tx/accounts/{public_key}/incoming_balances")]
pub async fn get_incoming_balances(
  path: web::Path<String>,
  repo: Repository,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let public_key = path.into_inner();
  // Get the account.
  let account_with_secret = repo
    .get_account_with_secret(&public_key)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?;

  let account = account_with_secret.as_confidential_account()?;

  // Get all assets with incoming balances for this account.
  let incoming = api
    .paged_query()
    .confidential_asset()
    .incoming_balance(account)
    .entries();
  pin!(incoming);
  let mut assets = Vec::new();
  while let Some(incoming) = incoming.next().await {
    match incoming {
      Ok((asset_id, Some(amount))) => {
        let enc_amount = scale_convert(&amount);
        let amount = account_with_secret.decrypt(&enc_amount)?;
        assets.push(AccountAssetIncomingBalance {
          asset_id: Uuid::from_bytes(asset_id),
          incoming_amount: amount,
        });
      }
      Ok((_, None)) => (),
      Err(err) => {
        Err(Error::from(err))?;
      }
    }
  }

  Ok(HttpResponse::Ok().json(assets))
}

/// Apply any incoming balances to the confidential account and update the local database.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/accounts/{public_key}/apply_incoming_balances")]
pub async fn tx_apply_incoming_balances(
  path: web::Path<String>,
  req: web::Json<TransactionArgs>,
  repo: Repository,
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let public_key = path.into_inner();
  let mut signer = signing
    .get_signer(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))?;
  // Get the account.
  let account_with_secret = repo
    .get_account_with_secret(&public_key)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?;

  let account = account_with_secret.as_confidential_account()?;

  // Get all assets with incoming balances for this account.
  let incoming = api
    .paged_query()
    .confidential_asset()
    .incoming_balance(account)
    .keys();
  pin!(incoming);
  let mut assets = Vec::new();
  let mut calls = Vec::new();
  while let Some(asset_id) = incoming.next().await {
    let asset_id = asset_id.map_err(|err| Error::from(err))?;
    assets.push(Uuid::from_bytes(asset_id));
    calls.push(
      api
        .call()
        .confidential_asset()
        .apply_incoming_balance(account, asset_id)
        .map_err(|err| Error::from(err))?
        .into(),
    );
  }

  if calls.len() == 0 {
    Err(Error::other("No incoming balances to apply"))?;
  }

  let res = api
    .call()
    .utility()
    .batch_all(calls)
    .map_err(|err| Error::from(err))?
    .submit_and_watch(&mut signer)
    .await
    .map_err(|err| Error::from(err))?;

  // Wait for transaction results.
  let mut res = TransactionResult::wait_for_results(res, req.finalize).await?;

  // Update account balance.
  if res.success {
    if let Some(updates) = res.decrypt_balance_updates(&account_with_secret) {
      for (_asset_id, update) in updates {
        repo.update_account_asset(&update).await?;
      }
    }
  }

  Ok(HttpResponse::Ok().json(res))
}

/// Affirm confidential asset settlements as the sender/receiver/mediator.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/accounts/{public_key}/affirm_transactions")]
pub async fn tx_affirm_transactions(
  path: web::Path<String>,
  req: web::Json<AffirmTransactionsRequest>,
  repo: Repository,
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let public_key = path.into_inner();
  let mut signer = signing
    .get_signer(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))?;
  let account_with_secret = repo
    .get_account_with_secret(&public_key)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?;

  let mut affirms = Vec::new();

  for tx in &req.transactions {
    let transaction_id = tx.transaction_id;
    for leg in &tx.legs {
      let leg_id = leg.leg_id;
      let affirm_party = match (&leg.party, &leg.amounts) {
        (TransactionParty::Sender, None) => Err(Error::other("Missing asset amounts."))?,
        (TransactionParty::Sender, Some(amounts)) => {
          // Query the chain for Transaction Leg to get the receiver and auditors.
          let leg_details = api
            .query()
            .confidential_asset()
            .transaction_legs(transaction_id, leg_id)
            .await
            .map_err(|err| Error::from(err))?
            .ok_or_else(|| Error::not_found("Transaction Leg"))?;

          let receiver = confidential_account_to_key(&leg_details.receiver);
          let sender = leg_details.sender;

          let mut transfers = ConfidentialTransfers {
            proofs: Default::default(),
          };

          if leg_details.auditors.len() != amounts.len() {
            Err(Error::other("Wrong number of asset amounts."))?
          }

          for amount in amounts {
            let asset_id = amount.asset_id;
            let amount = amount.amount;
            let auditors = leg_details
              .auditors
              .get(asset_id.as_bytes())
              .ok_or_else(|| Error::other(&format!("Invalid asset in leg: {asset_id:?}")))?;
            // Get the account asset with account secret key.
            let account_asset = repo
              .get_account_asset_with_secret(&public_key, asset_id)
              .await?
              .ok_or_else(|| Error::not_found("Account Asset"))?;
            let auditors = auditors.iter().map(auditor_account_to_key).collect();

            // Query the chain for the sender's current balance.
            let enc_balance = api
              .query()
              .confidential_asset()
              .account_balance(sender, *asset_id.as_bytes())
              .await
              .map_err(|err| Error::from(err))?
              .ok_or_else(|| Error::not_found("Sender account balance"))?;
            // Convert from on-chain `CipherText`.
            let enc_balance = Some(scale_convert(&enc_balance));

            // Generate sender proof.
            let (_update, proof) =
              account_asset.create_send_proof(enc_balance, receiver, auditors, amount)?;
            transfers
              .proofs
              .insert(*asset_id.as_bytes(), SenderProof(proof.as_bytes()));
          }
          AffirmParty::Sender(transfers)
        }
        (TransactionParty::Receiver, _amounts) => AffirmParty::Receiver,
        (TransactionParty::Mediator, _amounts) => AffirmParty::Mediator,
      };
      affirms.push(AffirmTransaction {
        id: transaction_id,
        leg: AffirmLeg {
          leg_id: leg_id,
          party: affirm_party,
        },
      });
    }
  }

  let res = api
    .call()
    .confidential_asset()
    .affirm_transactions(AffirmTransactions(affirms))
    .map_err(|err| Error::from(err))?
    .submit_and_watch(&mut signer)
    .await
    .map_err(|err| Error::from(err))?;

  // Wait for transaction results.
  let mut res = TransactionResult::wait_for_results(res, req.finalize).await?;

  // Update account balance.
  if res.success {
    if let Some(updates) = res.decrypt_balance_updates(&account_with_secret) {
      for (_asset_id, update) in updates {
        repo.update_account_asset(&update).await?;
      }
    }
  }

  Ok(HttpResponse::Ok().json(res))
}

/// Affirm confidential asset settlement as a mediator.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/accounts/{public_key}/mediator_affirm_leg")]
pub async fn tx_mediator_affirm_leg(
  path: web::Path<String>,
  req: web::Json<AffirmTransactionLegRequest>,
  repo: Repository,
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let public_key = path.into_inner();
  let mut signer = signing
    .get_signer(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))?;
  let _account = repo
    .get_account(&public_key)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?
    .as_auditor_account()?;

  let affirms = AffirmTransactions(vec![AffirmTransaction {
    id: req.transaction_id,
    leg: AffirmLeg {
      leg_id: req.leg_id,
      party: AffirmParty::Mediator,
    },
  }]);
  let res = api
    .call()
    .confidential_asset()
    .affirm_transactions(affirms)
    .map_err(|err| Error::from(err))?
    .submit_and_watch(&mut signer)
    .await
    .map_err(|err| Error::from(err))?;

  // Wait for transaction results.
  let res = TransactionResult::wait_for_results(res, req.finalize).await?;

  Ok(HttpResponse::Ok().json(res))
}
