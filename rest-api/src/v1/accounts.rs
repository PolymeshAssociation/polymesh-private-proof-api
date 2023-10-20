use actix_web::{get, post, web, HttpResponse, Responder, Result};

use polymesh_api::client::PairSigner;
use polymesh_api::types::pallet_confidential_asset::{AffirmLeg, AffirmParty};
use polymesh_api::Api;

use confidential_proof_shared::{
  error::Error, AffirmTransactionLegRequest, AuditorVerifyRequest, CreateAccount, TransactionArgs,
  TransactionResult,
};

use super::account_assets;
use crate::repo::Repository;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg
    .service(get_all_accounts)
    .service(get_account)
    .service(create_account)
    .service(tx_add_mediator)
    .service(tx_mediator_affirm_leg)
    .service(auditor_verify_request)
    .configure(account_assets::service);
}

/// Get all accounts.
#[utoipa::path(
  responses(
    (status = 200, body = [Account])
  )
)]
#[get("/accounts")]
pub async fn get_all_accounts(repo: web::Data<Repository>) -> Result<impl Responder> {
  let accounts = repo.get_accounts().await?;
  Ok(HttpResponse::Ok().json(accounts))
}

/// Get one account.
#[utoipa::path(
  responses(
    (status = 200, body = Account)
  )
)]
#[get("/accounts/{account_id}")]
pub async fn get_account(
  account_id: web::Path<i64>,
  repo: web::Data<Repository>,
) -> Result<impl Responder> {
  let account = repo
    .get_account(*account_id)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?;
  Ok(HttpResponse::Ok().json(account))
}

/// Create a new account.
#[utoipa::path(
  responses(
    (status = 200, body = Account)
  )
)]
#[post("/accounts")]
pub async fn create_account(repo: web::Data<Repository>) -> Result<impl Responder> {
  let account = CreateAccount::new();
  let account = repo.create_account(&account).await?;
  Ok(HttpResponse::Ok().json(account))
}

/// Add the account as a mediator on-chain.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/accounts/{account_id}/tx_add_mediator")]
pub async fn tx_add_mediator(
  account_id: web::Path<i64>,
  req: web::Json<TransactionArgs>,
  repo: web::Data<Repository>,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let mut signer = repo
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
#[post("/accounts/{account_id}/tx/mediator_affirm_leg")]
pub async fn tx_mediator_affirm_leg(
  path: web::Path<i64>,
  req: web::Json<AffirmTransactionLegRequest>,
  repo: web::Data<Repository>,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let account_id = path.into_inner();
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

/// Verify a sender proof as an auditor.
#[utoipa::path(
  responses(
    (status = 200, body = SenderProofVerifyResult)
  )
)]
#[post("/accounts/{account_id}/auditor_verify")]
pub async fn auditor_verify_request(
  account_id: web::Path<i64>,
  req: web::Json<AuditorVerifyRequest>,
  repo: web::Data<Repository>,
) -> Result<impl Responder> {
  // Get the account with secret key.
  let account = repo
    .get_account_with_secret(*account_id)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?;

  // Verify the sender's proof.
  let res = account.auditor_verify_proof(&req)?;
  Ok(HttpResponse::Ok().json(res))
}
