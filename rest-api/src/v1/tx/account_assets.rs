use actix_web::{get, post, web, HttpResponse, Responder, Result};
use uuid::Uuid;

use polymesh_api::types::{
  confidential_assets::transaction::ConfidentialTransferProof as SenderProof,
  pallet_confidential_asset::{
    AffirmLeg, AffirmParty, AffirmTransaction, AffirmTransactions, ConfidentialTransfers,
  },
};
use polymesh_api::Api;

use confidential_proof_api::repo::Repository;
use confidential_proof_shared::{
  auditor_account_to_key, confidential_account_to_key, error::Error, scale_convert,
  AffirmTransactionLegRequest, DecryptedIncomingBalance, MintRequest, TransactionArgs,
  TransactionResult,
};

use crate::signing::AppSigningManager;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg
    .service(tx_sender_affirm_leg)
    .service(tx_receiver_affirm_leg)
    .service(tx_apply_incoming)
    .service(get_incoming_balance)
    .service(tx_mint);
}

/// Affirm confidential asset settlement leg as the receiver.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/accounts/{public_key}/assets/{asset_id}/receiver_affirm_leg")]
pub async fn tx_receiver_affirm_leg(
  path: web::Path<(String, Uuid)>,
  req: web::Json<AffirmTransactionLegRequest>,
  repo: Repository,
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let (public_key, _asset_id) = path.into_inner();
  let mut signer = signing
    .get_signer(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))?;
  // Get the account.
  let _account = repo
    .get_account(&public_key)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?
    .as_confidential_account()?;

  let transaction_id = req.transaction_id;
  let leg_id = req.leg_id;

  let affirms = AffirmTransactions(vec![AffirmTransaction {
    id: transaction_id,
    leg: AffirmLeg {
      leg_id: leg_id,
      party: AffirmParty::Receiver,
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

/// Query chain for an account's incoming balance.
#[utoipa::path(
  responses(
    (status = 200, body = DecryptedIncomingBalance)
  )
)]
#[get("/tx/accounts/{public_key}/assets/{asset_id}/incoming_balance")]
pub async fn get_incoming_balance(
  path: web::Path<(String, Uuid)>,
  repo: Repository,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let (public_key, asset_id) = path.into_inner();
  // Get the account.
  let account_with_secret = repo
    .get_account_with_secret(&public_key)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?;

  let account = account_with_secret.as_confidential_account()?;
  // Get incoming balance.
  let enc_incoming = api
    .query()
    .confidential_asset()
    .incoming_balance(account, *asset_id.as_bytes())
    .await
    .map_err(|err| Error::from(err))?
    .map(|enc| scale_convert(&enc));

  // Decrypt incoming balance.
  let incoming_balance = if let Some(enc_incoming) = enc_incoming {
    Some(account_with_secret.decrypt(&enc_incoming)?)
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
#[post("/tx/accounts/{public_key}/assets/{asset_id}/apply_incoming")]
pub async fn tx_apply_incoming(
  path: web::Path<(String, Uuid)>,
  req: web::Json<TransactionArgs>,
  repo: Repository,
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let (public_key, asset_id) = path.into_inner();
  let mut signer = signing
    .get_signer(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))?;
  // Get the account.
  let account_with_secret = repo
    .get_account_with_secret(&public_key)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?;
  // Get the account asset with account secret key.
  let account_asset = repo
    .get_account_asset_with_secret(&public_key, asset_id)
    .await?;

  let account = account_with_secret.as_confidential_account()?;
  // Get pending incoming balance.
  let incoming_balance = api
    .query()
    .confidential_asset()
    .incoming_balance(account, *asset_id.as_bytes())
    .await
    .map_err(|err| Error::from(err))?
    .ok_or_else(|| Error::other("No incoming balance"))?;
  // Convert from on-chain `CipherText`.
  let enc_incoming = scale_convert(&incoming_balance);
  let update = match account_asset {
    Some(account_asset) => account_asset.apply_incoming(enc_incoming),
    None => account_with_secret.apply_incoming(asset_id, enc_incoming),
  }?;

  let res = api
    .call()
    .confidential_asset()
    .apply_incoming_balance(account, *asset_id.as_bytes())
    .map_err(|err| Error::from(err))?
    .submit_and_watch(&mut signer)
    .await
    .map_err(|err| Error::from(err))?;

  // Wait for transaction results.
  let res = TransactionResult::wait_for_results(res, req.finalize).await?;

  // Update account balance.
  if res.success {
    repo.update_account_asset(&update).await?;
  }

  Ok(HttpResponse::Ok().json(res))
}

/// Affirm confidential asset settlement leg as the sender.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/accounts/{public_key}/assets/{asset_id}/sender_affirm_leg")]
pub async fn tx_sender_affirm_leg(
  path: web::Path<(String, Uuid)>,
  req: web::Json<AffirmTransactionLegRequest>,
  repo: Repository,
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let (public_key, asset_id) = path.into_inner();
  let mut signer = signing
    .get_signer(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))?;
  // Get the account asset with account secret key.
  let account_asset = repo
    .get_account_asset_with_secret(&public_key, asset_id)
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

  let mut updates = Vec::new();
  let mut transfers = ConfidentialTransfers {
    proofs: Default::default(),
  };

  for (asset_id, auditors) in leg.auditors {
    let auditors = auditors.iter().map(auditor_account_to_key).collect();

    // Query the chain for the sender's current balance.
    let enc_balance = api
      .query()
      .confidential_asset()
      .account_balance(leg.sender, asset_id)
      .await
      .map_err(|err| Error::from(err))?
      .ok_or_else(|| Error::not_found("Sender account balance"))?;
    // Convert from on-chain `CipherText`.
    let enc_balance = Some(scale_convert(&enc_balance));

    // Generate sender proof.
    let (update, proof) =
      account_asset.create_send_proof(enc_balance, receiver, auditors, amount)?;
    transfers
      .proofs
      .insert(asset_id, SenderProof(proof.as_bytes()));
    updates.push(update);
  }

  let affirms = AffirmTransactions(vec![AffirmTransaction {
    id: transaction_id,
    leg: AffirmLeg {
      leg_id: leg_id,
      party: AffirmParty::Sender(transfers),
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

  // Update account balance.
  if res.success {
    for update in updates {
      repo.update_account_asset(&update).await?;
    }
  }

  Ok(HttpResponse::Ok().json(res))
}

/// Mint confidential assets on-chain.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/accounts/{public_key}/assets/{asset_id}/mint")]
pub async fn tx_mint(
  path: web::Path<(String, Uuid)>,
  req: web::Json<MintRequest>,
  repo: Repository,
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let (public_key, asset_id) = path.into_inner();
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
  let res = api
    .call()
    .confidential_asset()
    .mint(*asset_id.as_bytes(), req.amount as _, account)
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
