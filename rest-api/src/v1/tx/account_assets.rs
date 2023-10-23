use actix_web::{post, web, HttpResponse, Responder, Result};

use codec::Encode;

use polymesh_api::types::pallet_confidential_asset::{AffirmLeg, AffirmParty, SenderProof};
use polymesh_api::Api;

use confidential_proof_api::repo::Repository;
use confidential_proof_shared::{
  confidential_account_to_key, error::Error, mediator_account_to_key, scale_convert,
  AffirmTransactionLegRequest, MintRequest, TransactionArgs, TransactionResult,
};

use crate::signing::AppSigningManager;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg
    .service(tx_init_account)
    .service(tx_sender_affirm_leg)
    .service(tx_receiver_affirm_leg)
    .service(tx_apply_incoming)
    .service(tx_mint);
}

/// Add the account on-chain and initialize it's balance.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/accounts/{account_id}/assets/{asset_id}/init_account")]
pub async fn tx_init_account(
  path: web::Path<(i64, i64)>,
  req: web::Json<TransactionArgs>,
  repo: Repository,
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let (account_id, asset_id) = path.into_inner();
  let mut signer = signing
    .get_signer(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))?;
  // Get the account.
  let account = repo
    .get_account(account_id)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?
    .as_confidential_account()?;
  let ticker = repo
    .get_asset(asset_id)
    .await?
    .ok_or_else(|| Error::not_found("Asset"))?
    .ticker()?;

  let res = api
    .call()
    .confidential_asset()
    .create_account(ticker, account)
    .map_err(|err| Error::from(err))?
    .submit_and_watch(&mut signer)
    .await
    .map_err(|err| Error::from(err))?;

  // Wait for transaction results.
  let res = TransactionResult::wait_for_results(res, req.finalize).await?;

  Ok(HttpResponse::Ok().json(res))
}

/// Affirm confidential asset settlement leg as the receiver.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/accounts/{account_id}/assets/{asset_id}/receiver_affirm_leg")]
pub async fn tx_receiver_affirm_leg(
  path: web::Path<(i64, i64)>,
  req: web::Json<AffirmTransactionLegRequest>,
  repo: Repository,
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let (account_id, asset_id) = path.into_inner();
  let mut signer = signing
    .get_signer(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))?;
  // Get the account asset with account secret key.
  let _account_asset = repo
    .get_account_asset_with_secret(account_id, asset_id)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;

  let transaction_id = req.transaction_id;
  let leg_id = req.leg_id;

  let affirm = AffirmLeg {
    leg_id,
    party: AffirmParty::Receiver,
  };
  let res = api
    .call()
    .confidential_asset()
    .affirm_transaction(transaction_id, affirm)
    .map_err(|err| Error::from(err))?
    .submit_and_watch(&mut signer)
    .await
    .map_err(|err| Error::from(err))?;

  // Wait for transaction results.
  let res = TransactionResult::wait_for_results(res, req.finalize).await?;

  Ok(HttpResponse::Ok().json(res))
}

/// Apply any incoming balance to the confidential account and update the local database.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/accounts/{account_id}/assets/{asset_id}/apply_incoming")]
pub async fn tx_apply_incoming(
  path: web::Path<(i64, i64)>,
  req: web::Json<TransactionArgs>,
  repo: Repository,
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let (account_id, asset_id) = path.into_inner();
  let mut signer = signing
    .get_signer(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))?;
  // Get the account.
  let account = repo
    .get_account(account_id)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?
    .as_confidential_account()?;
  let ticker = repo
    .get_asset(asset_id)
    .await?
    .ok_or_else(|| Error::not_found("Asset"))?
    .ticker()?;

  let res = api
    .call()
    .confidential_asset()
    .apply_incoming_balance(account, ticker)
    .map_err(|err| Error::from(err))?
    .submit_and_watch(&mut signer)
    .await
    .map_err(|err| Error::from(err))?;

  // Wait for transaction results.
  let res = TransactionResult::wait_for_results(res, req.finalize).await?;
  // TODO: Update balance in database.

  Ok(HttpResponse::Ok().json(res))
}

/// Affirm confidential asset settlement leg as the sender.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/accounts/{account_id}/assets/{asset_id}/sender_affirm_leg")]
pub async fn tx_sender_affirm_leg(
  path: web::Path<(i64, i64)>,
  req: web::Json<AffirmTransactionLegRequest>,
  repo: Repository,
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let (account_id, asset_id) = path.into_inner();
  let mut signer = signing
    .get_signer(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))?;
  // Get the account asset with account secret key.
  let account_asset = repo
    .get_account_asset_with_secret(account_id, asset_id)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;

  let transaction_id = req.transaction_id;
  let leg_id = req.leg_id;
  let amount = req.amount;

  // Query the chain for Transaction Leg to get the receiver and auditors.
  let leg = api
    .query()
    .confidential_asset()
    .transaction_legs(transaction_id, leg_id)
    .await
    .map_err(|err| Error::from(err))?
    .ok_or_else(|| Error::not_found("Transaction Leg"))?;
  let receiver = confidential_account_to_key(&leg.receiver);
  let auditors = leg
    .auditors
    .auditors
    .keys()
    .into_iter()
    .map(mediator_account_to_key)
    .collect();

  // Query the chain for the sender's current balance.
  let enc_balance = api
    .query()
    .confidential_asset()
    .account_balance(leg.sender, leg.ticker)
    .await
    .map_err(|err| Error::from(err))?
    .ok_or_else(|| Error::not_found("Sender account balance"))?;
  // Convert from on-chain `CipherText`.
  let enc_balance = Some(scale_convert(&enc_balance));

  // Generate sender proof.
  let (update, proof) = account_asset.create_send_proof(enc_balance, receiver, auditors, amount)?;

  let affirm = AffirmLeg {
    leg_id,
    party: AffirmParty::Sender(Box::new(SenderProof(proof.encode()))),
  };
  let res = api
    .call()
    .confidential_asset()
    .affirm_transaction(transaction_id, affirm)
    .map_err(|err| Error::from(err))?
    .submit_and_watch(&mut signer)
    .await
    .map_err(|err| Error::from(err))?;

  // Wait for transaction results.
  let res = TransactionResult::wait_for_results(res, req.finalize).await?;

  // Update account balance.
  if res.success {
    repo
      .update_account_asset(&update)
      .await?
      .ok_or_else(|| Error::not_found("Account Asset"))?;
  }

  Ok(HttpResponse::Ok().json(res))
}

/// Mint confidential assets on-chain.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/accounts/{account_id}/assets/{asset_id}/mint")]
pub async fn tx_mint(
  path: web::Path<(i64, i64)>,
  req: web::Json<MintRequest>,
  repo: Repository,
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let (account_id, asset_id) = path.into_inner();
  let mut signer = signing
    .get_signer(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))?;
  // Get the account.
  let account = repo
    .get_account(account_id)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?
    .as_confidential_account()?;
  let ticker = repo
    .get_asset(asset_id)
    .await?
    .ok_or_else(|| Error::not_found("Asset"))?
    .ticker()?;

  let res = api
    .call()
    .confidential_asset()
    .mint_confidential_asset(ticker, req.amount as _, account)
    .map_err(|err| Error::from(err))?
    .submit_and_watch(&mut signer)
    .await
    .map_err(|err| Error::from(err))?;

  // Wait for transaction results.
  let res = TransactionResult::wait_for_results(res, req.finalize).await?;

  // TODO: Update balance

  Ok(HttpResponse::Ok().json(res))
}
