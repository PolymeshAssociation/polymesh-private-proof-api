use actix_web::{get, post, web, HttpResponse, Responder, Result};

use codec::Encode;

use polymesh_api::client::PairSigner;
use polymesh_api::types::pallet_confidential_asset::{AffirmLeg, AffirmParty, SenderProof};
use polymesh_api::Api;

use confidential_proof_shared::{
  confidential_account_to_key, error::Error, mediator_account_to_key, scale_convert,
  AccountAssetWithProof, AccountMintAsset, AffirmTransactionLegRequest, CreateAccountAsset,
  MintRequest, ReceiverVerifyRequest, SenderProofRequest, SenderProofVerifyResult, TransactionArgs,
  TransactionResult, UpdateAccountAssetBalanceRequest,
};

use crate::repo::Repository;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg
    .service(get_all_account_assets)
    .service(get_account_asset)
    .service(create_account_asset)
    .service(tx_init_account)
    .service(tx_sender_affirm_leg)
    .service(tx_receiver_affirm_leg)
    .service(tx_apply_incoming)
    .service(tx_mint)
    .service(asset_issuer_mint)
    .service(request_sender_proof)
    .service(receiver_verify_request)
    .service(update_balance_request);
}

/// Get all assets for an account.
#[utoipa::path(
  responses(
    (status = 200, body = [AccountAsset])
  )
)]
#[get("/accounts/{account_id}/assets")]
pub async fn get_all_account_assets(
  account_id: web::Path<i64>,
  repo: web::Data<Repository>,
) -> Result<impl Responder> {
  let account_assets = repo.get_account_assets(*account_id).await?;
  Ok(HttpResponse::Ok().json(account_assets))
}

/// Get one asset for the account.
#[utoipa::path(
  responses(
    (status = 200, body = AccountAsset)
  )
)]
#[get("/accounts/{account_id}/assets/{asset_id}")]
pub async fn get_account_asset(
  path: web::Path<(i64, i64)>,
  repo: web::Data<Repository>,
) -> Result<impl Responder> {
  let (account_id, asset_id) = path.into_inner();
  let account_asset = repo
    .get_account_asset(account_id, asset_id)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;
  Ok(HttpResponse::Ok().json(account_asset))
}

/// Add an asset to the account and initialize it's balance.
#[utoipa::path(
  responses(
    (status = 200, body = AccountAsset)
  )
)]
#[post("/accounts/{account_id}/assets")]
pub async fn create_account_asset(
  account_id: web::Path<i64>,
  create_account_asset: web::Json<CreateAccountAsset>,
  repo: web::Data<Repository>,
) -> Result<impl Responder> {
  // Get the account's secret key.
  let account = repo
    .get_account_with_secret(*account_id)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?;

  // Generate Account initialization proof.
  let init = account.init_balance(create_account_asset.asset_id);

  // Save initialize account balance.
  let account_asset = repo.create_account_asset(&init).await?;

  // Return account_asset.
  Ok(HttpResponse::Ok().json(account_asset))
}

/// Add the account on-chain and initialize it's balance.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/accounts/{account_id}/assets/{asset_id}/tx/init_account")]
pub async fn tx_init_account(
  path: web::Path<(i64, i64)>,
  req: web::Json<TransactionArgs>,
  repo: web::Data<Repository>,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let (account_id, asset_id) = path.into_inner();
  let mut signer = repo
    .get_signer_with_secret(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))
    .and_then(|signer| Ok(PairSigner::new(signer.keypair()?)))?;
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
#[post("/accounts/{account_id}/assets/{asset_id}/tx/receiver_affirm_leg")]
pub async fn tx_receiver_affirm_leg(
  path: web::Path<(i64, i64)>,
  req: web::Json<AffirmTransactionLegRequest>,
  repo: web::Data<Repository>,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let (account_id, asset_id) = path.into_inner();
  let mut signer = repo
    .get_signer_with_secret(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))
    .and_then(|signer| Ok(PairSigner::new(signer.keypair()?)))?;
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
#[post("/accounts/{account_id}/assets/{asset_id}/tx/apply_incoming")]
pub async fn tx_apply_incoming(
  path: web::Path<(i64, i64)>,
  req: web::Json<TransactionArgs>,
  repo: web::Data<Repository>,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let (account_id, asset_id) = path.into_inner();
  let mut signer = repo
    .get_signer_with_secret(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))
    .and_then(|signer| Ok(PairSigner::new(signer.keypair()?)))?;
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
#[post("/accounts/{account_id}/assets/{asset_id}/tx/sender_affirm_leg")]
pub async fn tx_sender_affirm_leg(
  path: web::Path<(i64, i64)>,
  req: web::Json<AffirmTransactionLegRequest>,
  repo: web::Data<Repository>,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let (account_id, asset_id) = path.into_inner();
  let mut signer = repo
    .get_signer_with_secret(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))
    .and_then(|signer| Ok(PairSigner::new(signer.keypair()?)))?;
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
#[post("/accounts/{account_id}/assets/{asset_id}/tx/mint")]
pub async fn tx_mint(
  path: web::Path<(i64, i64)>,
  req: web::Json<MintRequest>,
  repo: web::Data<Repository>,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let (account_id, asset_id) = path.into_inner();
  let mut signer = repo
    .get_signer_with_secret(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))
    .and_then(|signer| Ok(PairSigner::new(signer.keypair()?)))?;
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

/// Asset issuer updates their account balance when minting.
#[utoipa::path(
  responses(
    (status = 200, body = AccountAsset)
  )
)]
#[post("/accounts/{account_id}/assets/{asset_id}/mint")]
pub async fn asset_issuer_mint(
  path: web::Path<(i64, i64)>,
  account_mint_asset: web::Json<AccountMintAsset>,
  repo: web::Data<Repository>,
) -> Result<impl Responder> {
  let (account_id, asset_id) = path.into_inner();
  // Get the account asset with account secret key.
  let account_asset = repo
    .get_account_asset_with_secret(account_id, asset_id)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;

  // Mint asset.
  let update = account_asset.mint(account_mint_asset.amount)?;

  // Update account balance.
  let account_asset = repo
    .update_account_asset(&update)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;

  // Return account_asset.
  Ok(HttpResponse::Ok().json(account_asset))
}

/// Generate a sender proof.
#[utoipa::path(
  responses(
    (status = 200, body = AccountAssetWithProof)
  )
)]
#[post("/accounts/{account_id}/assets/{asset_id}/send")]
pub async fn request_sender_proof(
  path: web::Path<(i64, i64)>,
  req: web::Json<SenderProofRequest>,
  repo: web::Data<Repository>,
) -> Result<impl Responder> {
  let (account_id, asset_id) = path.into_inner();
  // Get the account asset with account secret key.
  let account_asset = repo
    .get_account_asset_with_secret(account_id, asset_id)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;

  let enc_balance = req.encrypted_balance()?;
  let receiver = req.receiver()?;
  let auditors = req.auditors()?;
  let amount = req.amount;

  // Generate sender proof.
  let (update, proof) = account_asset.create_send_proof(enc_balance, receiver, auditors, amount)?;

  // Update account balance.
  let account_asset = repo
    .update_account_asset(&update)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;

  // Return account_asset with sender proof.
  let balance_with_proof = AccountAssetWithProof::new_send_proof(account_asset, proof);
  Ok(HttpResponse::Ok().json(balance_with_proof))
}

/// Verify a sender proof as the receiver.
#[utoipa::path(
  responses(
    (status = 200, body = SenderProofVerifyResult)
  )
)]
#[post("/accounts/{account_id}/assets/{asset_id}/receiver_verify")]
pub async fn receiver_verify_request(
  path: web::Path<(i64, i64)>,
  req: web::Json<ReceiverVerifyRequest>,
  repo: web::Data<Repository>,
) -> Result<impl Responder> {
  let (account_id, asset_id) = path.into_inner();
  // Get the account asset with account secret key.
  let account_asset = repo
    .get_account_asset_with_secret(account_id, asset_id)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;

  // Verify the sender's proof.
  let res = account_asset.receiver_verify_proof(&req);
  Ok(HttpResponse::Ok().json(SenderProofVerifyResult::from_result(res)))
}

/// Update an account's encrypted balance.
#[utoipa::path(
  responses(
    (status = 200, body = bool)
  )
)]
#[post("/accounts/{account_id}/assets/{asset_id}/update_balance")]
pub async fn update_balance_request(
  path: web::Path<(i64, i64)>,
  req: web::Json<UpdateAccountAssetBalanceRequest>,
  repo: web::Data<Repository>,
) -> Result<impl Responder> {
  let (account_id, asset_id) = path.into_inner();
  // Get the account asset with account secret key.
  let account_asset = repo
    .get_account_asset_with_secret(account_id, asset_id)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;

  // Prepare balance update.
  let update = account_asset.update_balance(&req)?;

  // Update account balance.
  let account_asset = repo
    .update_account_asset(&update)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;

  // Return account_asset.
  Ok(HttpResponse::Ok().json(account_asset))
}
