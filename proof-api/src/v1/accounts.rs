use actix_web::{get, post, web, HttpResponse, Responder, Result};

use confidential_proof_shared::{error::Error, AuditorVerifyRequest, CreateAccount};

use super::account_assets;
use crate::repo::Repository;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg
    .service(get_all_accounts)
    .service(get_account)
    .service(create_account)
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
pub async fn get_account(account_id: web::Path<i64>, repo: web::Data<Repository>) -> Result<impl Responder> {
  let account = repo.get_account(*account_id).await?
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

/// Verify a sender proof as an auditor.
#[utoipa::path(
  responses(
    (status = 200, body = bool)
  )
)]
#[post("/accounts/{account_id}/auditor_verify")]
pub async fn auditor_verify_request(
  account_id: web::Path<i64>,
  req: web::Json<AuditorVerifyRequest>,
  repo: web::Data<Repository>,
) -> Result<impl Responder> {
  // Get the account with secret key.
  let account = repo.get_account_with_secret(*account_id).await?
    .ok_or_else(|| Error::not_found("Account"))?;

  // Verify the sender's proof.
  Ok(match account.auditor_verify_proof(&req) {
    Ok(is_valid) => {
      HttpResponse::Ok().json(is_valid)
    }
    Err(e) => {
      HttpResponse::InternalServerError()
        .body(format!("Sender proof verification failed: {e:?}"))
    }
  })
}
