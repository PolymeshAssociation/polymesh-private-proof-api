use actix_web::{post, web, HttpResponse, Responder, Result};

use polymesh_api::client::PairSigner;
use polymesh_api::types::pallet_confidential_asset::{AffirmLeg, AffirmParty};
use polymesh_api::Api;

use confidential_proof_api::repo::Repository;
use confidential_proof_shared::{
  error::Error, AffirmTransactionLegRequest, TransactionArgs, TransactionResult,
};

use super::account_assets;
use crate::signing::SigningManager;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg
    .service(tx_add_mediator)
    .service(tx_mediator_affirm_leg)
    .configure(account_assets::service);
}

/// Add the account as a mediator on-chain.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/accounts/{account_id}/add_mediator")]
pub async fn tx_add_mediator(
  account_id: web::Path<i64>,
  req: web::Json<TransactionArgs>,
  repo: web::Data<Repository>,
  signing: web::Data<SigningManager>,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let mut signer = signing
    .get_signer_with_secret(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))
    .and_then(|signer| Ok(PairSigner::new(signer.keypair()?)))?;
  // Get the account.
  let account = repo
    .get_account(*account_id)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?
    .as_mediator_account()?;

  let res = api
    .call()
    .confidential_asset()
    .add_mediator_account(account)
    .map_err(|err| Error::from(err))?
    .submit_and_watch(&mut signer)
    .await
    .map_err(|err| Error::from(err))?;

  // Wait for transaction results.
  let res = TransactionResult::wait_for_results(res, req.finalize).await?;

  Ok(HttpResponse::Ok().json(res))
}

/// Affirm confidential asset settlement as a mediator.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/accounts/{account_id}/mediator_affirm_leg")]
pub async fn tx_mediator_affirm_leg(
  path: web::Path<i64>,
  req: web::Json<AffirmTransactionLegRequest>,
  repo: web::Data<Repository>,
  signing: web::Data<SigningManager>,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let account_id = path.into_inner();
  let mut signer = signing
    .get_signer_with_secret(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))
    .and_then(|signer| Ok(PairSigner::new(signer.keypair()?)))?;
  // Get the account.
  let account = repo
    .get_account(account_id)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?
    .as_mediator_account()?;

  let transaction_id = req.transaction_id;
  let leg_id = req.leg_id;
  let affirm = AffirmLeg {
    leg_id,
    party: AffirmParty::Mediator(account),
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
