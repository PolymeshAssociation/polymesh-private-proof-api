use actix_web::{post, web, HttpResponse, Responder, Result};

use polymesh_api::types::pallet_confidential_asset::{
  AffirmLeg, AffirmParty, AffirmTransaction, AffirmTransactions,
};
use polymesh_api::Api;

use confidential_proof_api::repo::Repository;
use confidential_proof_shared::{
  error::Error, AffirmTransactionLegRequest, PublicKey, TransactionArgs, TransactionResult,
};

use super::account_assets;
use crate::signing::AppSigningManager;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg
    .service(tx_init_account)
    .service(tx_account_did)
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
