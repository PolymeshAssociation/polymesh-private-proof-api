use actix_web::{get, post, web, HttpResponse, Responder, Result};

use codec::Encode;

use polymesh_api::types::pallet_confidential_asset::{AffirmLeg, AffirmParty, SenderProof};
use polymesh_api::Api;

use confidential_proof_api::repo::Repository;
use confidential_proof_shared::{
  confidential_account_to_key, error::Error, mediator_account_to_key, scale_convert, str_to_ticker,
  AffirmTransactionLegRequest, DecryptedIncomingBalance, MintRequest, PublicKey, TransactionArgs,
  TransactionResult,
};

use crate::signing::AppSigningManager;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg
    .service(tx_init_account)
    .service(tx_sender_affirm_leg)
    .service(tx_receiver_affirm_leg)
    .service(tx_apply_incoming)
    .service(get_incoming_balance)
    .service(tx_mint);
}

/// Add the account on-chain and initialize it's balance.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/accounts/{public_key}/assets/{ticker}/init_account")]
pub async fn tx_init_account(
  path: web::Path<(PublicKey, String)>,
  req: web::Json<TransactionArgs>,
  repo: Repository,
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let (public_key, ticker) = path.into_inner();
  let ticker = str_to_ticker(&ticker)?;
  let mut signer = signing
    .get_signer(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))?;
  // Get the account.
  let account = repo
    .get_account(&public_key)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?
    .as_confidential_account()?;

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
#[post("/tx/accounts/{public_key}/assets/{ticker}/receiver_affirm_leg")]
pub async fn tx_receiver_affirm_leg(
  path: web::Path<(PublicKey, String)>,
  req: web::Json<AffirmTransactionLegRequest>,
  repo: Repository,
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let (public_key, ticker) = path.into_inner();
  let mut signer = signing
    .get_signer(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))?;
  // Get the account asset with account secret key.
  let _account_asset = repo
    .get_account_asset_with_secret(&public_key, &ticker)
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

/// Query chain for an account's incoming balance.
#[utoipa::path(
  responses(
    (status = 200, body = DecryptedIncomingBalance)
  )
)]
#[get("/tx/accounts/{public_key}/assets/{ticker}/incoming_balance")]
pub async fn get_incoming_balance(
  path: web::Path<(PublicKey, String)>,
  repo: Repository,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let (public_key, ticker) = path.into_inner();
  // Get the account.
  let account = repo
    .get_account(&public_key)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?
    .as_confidential_account()?;
  // Get the account asset with account secret key.
  let account_asset = repo
    .get_account_asset_with_secret(&public_key, &ticker)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;
  let ticker = str_to_ticker(&ticker)?;

  // Get incoming balance.
  let enc_incoming = api
    .query()
    .confidential_asset()
    .incoming_balance(account, ticker)
    .await
    .map_err(|err| Error::from(err))?
    .map(|enc| scale_convert(&enc));

  // Decrypt incoming balance.
  let incoming_balance = if let Some(enc_incoming) = enc_incoming {
    Some(account_asset.decrypt(&enc_incoming)?)
  } else {
    None
  };

  Ok(HttpResponse::Ok().json(DecryptedIncomingBalance { incoming_balance }))
}

/// Apply any incoming balance to the confidential account and update the local database.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/accounts/{public_key}/assets/{ticker}/apply_incoming")]
pub async fn tx_apply_incoming(
  path: web::Path<(PublicKey, String)>,
  req: web::Json<TransactionArgs>,
  repo: Repository,
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let (public_key, ticker) = path.into_inner();
  let mut signer = signing
    .get_signer(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))?;
  // Get the account.
  let account = repo
    .get_account(&public_key)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?
    .as_confidential_account()?;
  // Get the account asset with account secret key.
  let account_asset = repo
    .get_account_asset_with_secret(&public_key, &ticker)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;
  let ticker = str_to_ticker(&ticker)?;

  // Get pending incoming balance.
  let incoming_balance = api
    .query()
    .confidential_asset()
    .incoming_balance(account, ticker)
    .await
    .map_err(|err| Error::from(err))?
    .ok_or_else(|| Error::other("No incoming balance"))?;
  // Convert from on-chain `CipherText`.
  let enc_incoming = scale_convert(&incoming_balance);
  let update = account_asset.apply_incoming(enc_incoming)?;

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

  // Update account balance.
  if res.success {
    repo
      .update_account_asset(&update)
      .await?
      .ok_or_else(|| Error::not_found("Account Asset"))?;
  }

  Ok(HttpResponse::Ok().json(res))
}

/// Affirm confidential asset settlement leg as the sender.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/accounts/{public_key}/assets/{ticker}/sender_affirm_leg")]
pub async fn tx_sender_affirm_leg(
  path: web::Path<(PublicKey, String)>,
  req: web::Json<AffirmTransactionLegRequest>,
  repo: Repository,
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let (public_key, ticker) = path.into_inner();
  let mut signer = signing
    .get_signer(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))?;
  // Get the account asset with account secret key.
  let account_asset = repo
    .get_account_asset_with_secret(&public_key, &ticker)
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
#[post("/tx/accounts/{public_key}/assets/{ticker}/mint")]
pub async fn tx_mint(
  path: web::Path<(PublicKey, String)>,
  req: web::Json<MintRequest>,
  repo: Repository,
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let (public_key, ticker) = path.into_inner();
  let mut signer = signing
    .get_signer(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))?;
  // Get the account.
  let account = repo
    .get_account(&public_key)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?
    .as_confidential_account()?;
  // Get the account asset.
  let account_asset = repo
    .get_account_asset(&public_key, &ticker)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;
  let ticker = str_to_ticker(&ticker)?;

  // Prepare to update account balance.
  let update = account_asset.mint(req.amount)?;

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

  // Update account balance.
  if res.success {
    repo
      .update_account_asset(&update)
      .await?
      .ok_or_else(|| Error::not_found("Account Asset"))?;
  }

  Ok(HttpResponse::Ok().json(res))
}
