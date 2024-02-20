use actix_web::{get, post, web, HttpResponse, Responder, Result};

use confidential_proof_shared::{
  error::Error, AccountDecryptRequest, AuditorVerifyRequest, BurnProof, BurnProofRequest,
  CreateAccount, ReceiverVerifyRequest, SenderProof, SenderProofRequest,
};

use crate::repo::Repository;

pub fn service(cfg: &mut web::ServiceConfig) {
  let _cfg = cfg
    .service(get_all_accounts)
    .service(get_account)
    .service(create_account)
    .service(decrypt_request)
    .service(request_sender_proof)
    .service(request_burn_proof)
    .service(receiver_verify_request)
    .service(auditor_verify_request);

  #[cfg(feature = "track_balances")]
  _cfg.configure(super::account_assets::service);
}

/// Get all accounts.
#[utoipa::path(
  responses(
    (status = 200, body = [Account])
  )
)]
#[get("/accounts")]
pub async fn get_all_accounts(repo: Repository) -> Result<impl Responder> {
  let accounts = repo.get_accounts().await?;
  Ok(HttpResponse::Ok().json(accounts))
}

/// Get one account.
#[utoipa::path(
  responses(
    (status = 200, body = Account)
  )
)]
#[get("/accounts/{public_key}")]
pub async fn get_account(
  public_key: web::Path<String>,
  repo: Repository,
) -> Result<impl Responder> {
  let account = repo
    .get_account(&public_key)
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
pub async fn create_account(repo: Repository) -> Result<impl Responder> {
  let account = CreateAccount::new();
  let account = repo.create_account(&account).await?;
  Ok(HttpResponse::Ok().json(account))
}

/// Generate a sender proof.
#[utoipa::path(
  responses(
    (status = 200, body = SenderProof)
  )
)]
#[post("/accounts/{public_key}/send")]
pub async fn request_sender_proof(
  public_key: web::Path<String>,
  req: web::Json<SenderProofRequest>,
  repo: Repository,
) -> Result<impl Responder> {
  // Get the account asset with account secret key.
  let account = repo
    .get_account_with_secret(&public_key)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?;

  let enc_balance = req
    .encrypted_balance()?
    .ok_or_else(|| Error::other("Missing 'encrypted_balance'"))?;
  let receiver = req.receiver()?;
  let auditors = req.auditors()?;
  let amount = req.amount;

  // Generate sender proof.
  let proof = account.create_send_proof(enc_balance, None, receiver, auditors, amount)?;

  Ok(HttpResponse::Ok().json(SenderProof::new(proof)))
}

/// Verify a sender proof as the receiver.
#[utoipa::path(
  responses(
    (status = 200, body = SenderProofVerifyResult)
  )
)]
#[post("/accounts/{public_key}/receiver_verify")]
pub async fn receiver_verify_request(
  public_key: web::Path<String>,
  req: web::Json<ReceiverVerifyRequest>,
  repo: Repository,
) -> Result<impl Responder> {
  // Get the account asset with account secret key.
  let account = repo
    .get_account_with_secret(&public_key)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?;

  // Verify the sender's proof.
  let res = account.receiver_verify_proof(&req)?;
  Ok(HttpResponse::Ok().json(res))
}

/// Generate a burn proof.
#[utoipa::path(
  responses(
    (status = 200, body = BurnProof)
  )
)]
#[post("/accounts/{public_key}/burn")]
pub async fn request_burn_proof(
  public_key: web::Path<String>,
  req: web::Json<BurnProofRequest>,
  repo: Repository,
) -> Result<impl Responder> {
  // Get the account asset with account secret key.
  let account = repo
    .get_account_with_secret(&public_key)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?;

  let enc_balance = req
    .encrypted_balance()?
    .ok_or_else(|| Error::other("Missing 'encrypted_balance'"))?;
  let amount = req.amount;

  // Generate burn proof.
  let proof = account.create_burn_proof(enc_balance, None, amount)?;

  Ok(HttpResponse::Ok().json(BurnProof::new(proof)))
}

/// Decrypt a `CipherText` value.
#[utoipa::path(
  responses(
    (status = 200, body = DecryptedResponse)
  )
)]
#[post("/accounts/{public_key}/decrypt")]
pub async fn decrypt_request(
  public_key: web::Path<String>,
  req: web::Json<AccountDecryptRequest>,
  repo: Repository,
) -> Result<impl Responder> {
  // Get the account asset with account secret key.
  let account = repo
    .get_account_with_secret(&public_key)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?;

  // Decrypt the value.
  let resp = account.decrypt_request(&req)?;

  // Return the decrypted value.
  Ok(HttpResponse::Ok().json(resp))
}

/// Verify a sender proof as an auditor.
#[utoipa::path(
  responses(
    (status = 200, body = SenderProofVerifyResult)
  )
)]
#[post("/accounts/{public_key}/auditor_verify")]
pub async fn auditor_verify_request(
  public_key: web::Path<String>,
  req: web::Json<AuditorVerifyRequest>,
  repo: Repository,
) -> Result<impl Responder> {
  // Get the account with secret key.
  let account = repo
    .get_account_with_secret(&public_key)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?;

  // Verify the sender's proof.
  let res = account.auditor_verify_proof(&req)?;
  Ok(HttpResponse::Ok().json(res))
}
