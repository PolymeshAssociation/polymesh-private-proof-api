use actix_web::{web, HttpResponse, Responder, Result};

use confidential_assets_api_shared::{AuditorVerifyRequest, CreateAccount};

use super::account_assets;
use crate::repo::Repository;

fn account_service(cfg: &mut web::ServiceConfig) {
  cfg.service(
    web::scope("/{account_id}")
      // GET
      .route("", web::get().to(get_account))
      // POST
      .route("/auditor_verify", web::post().to(auditor_verify_request))
      .configure(account_assets::service),
  );
}

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg.service(
    web::scope("/accounts")
      // GET
      .route("", web::get().to(get_all_accounts))
      .configure(account_service)
      // POST
      .route("", web::post().to(create_account)),
  );
}

/// Get all accounts.
async fn get_all_accounts(repo: web::Data<Repository>) -> Result<impl Responder> {
  Ok(match repo.get_accounts().await {
    Ok(accounts) => HttpResponse::Ok().json(accounts),
    Err(e) => HttpResponse::NotFound().body(format!("Internal server error: {:?}", e)),
  })
}

/// Get one account.
async fn get_account(account_id: web::Path<i64>, repo: web::Data<Repository>) -> HttpResponse {
  match repo.get_account(*account_id).await {
    Ok(account) => HttpResponse::Ok().json(account),
    Err(_) => HttpResponse::NotFound().body("Not found"),
  }
}

/// Create a new account.
async fn create_account(repo: web::Data<Repository>) -> HttpResponse {
  let account = CreateAccount::new();
  match repo.create_account(&account).await {
    Ok(account) => HttpResponse::Ok().json(account),
    Err(e) => HttpResponse::InternalServerError().body(format!("Internal server error: {:?}", e)),
  }
}

/// Verify a sender proof as an auditor.
async fn auditor_verify_request(
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
