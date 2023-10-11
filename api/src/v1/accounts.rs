use actix_web::{get, post, web, HttpResponse, Responder, Result};

use confidential_assets_api_shared::{AuditorVerifyRequest, CreateAccount};

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
    (status = 200, description = "List all accounts", body = [Account])
  )
)]
#[get("/accounts")]
pub async fn get_all_accounts(repo: web::Data<Repository>) -> Result<impl Responder> {
  Ok(match repo.get_accounts().await {
    Ok(accounts) => HttpResponse::Ok().json(accounts),
    Err(e) => HttpResponse::NotFound().body(format!("Internal server error: {:?}", e)),
  })
}

/// Get one account.
#[utoipa::path(
  responses(
    (status = 200, description = "Get an account", body = Account)
  )
)]
#[get("/accounts/{account_id}")]
pub async fn get_account(account_id: web::Path<i64>, repo: web::Data<Repository>) -> HttpResponse {
  match repo.get_account(*account_id).await {
    Ok(account) => HttpResponse::Ok().json(account),
    Err(_) => HttpResponse::NotFound().body("Not found"),
  }
}

/// Create a new account.
#[utoipa::path(
  responses(
    (status = 200, description = "Create an account", body = Account)
  )
)]
#[post("/accounts")]
pub async fn create_account(repo: web::Data<Repository>) -> HttpResponse {
  let account = CreateAccount::new();
  match repo.create_account(&account).await {
    Ok(account) => HttpResponse::Ok().json(account),
    Err(e) => HttpResponse::InternalServerError().body(format!("Internal server error: {:?}", e)),
  }
}

/// Verify a sender proof as an auditor.
#[utoipa::path(
  responses(
    (status = 200, description = "Verify a sender proof as an auditor", body = bool)
  )
)]
#[post("/accounts/{account_id}/auditor_verify")]
pub async fn auditor_verify_request(
  account_id: web::Path<i64>,
  req: web::Json<AuditorVerifyRequest>,
  repo: web::Data<Repository>,
) -> HttpResponse {
  // Get the account with secret key.
  let account = match repo.get_account_with_secret(*account_id).await {
    Ok(account) => account,
    Err(_) => {
      return HttpResponse::NotFound().body("Account not found");
    }
  };

  // Verify the sender's proof.
  match account.auditor_verify_tx(&req) {
    Ok(is_valid) => {
      return HttpResponse::Ok().json(is_valid);
    }
    Err(e) => {
      return HttpResponse::InternalServerError()
        .body(format!("Sender proof verification failed: {e:?}"));
    }
  }
}
