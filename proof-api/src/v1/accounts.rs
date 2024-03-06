use actix_web::{get, post, web, HttpResponse, Responder, Result};

use polymesh-private-proof-shared::{
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

/// Get all confidential accounts.
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

/// Get one confidential account.
#[utoipa::path(
  responses(
    (status = 200, body = Account)
  )
)]
#[get("/accounts/{confidential_account}")]
pub async fn get_account(
  confidential_account: web::Path<String>,
  repo: Repository,
) -> Result<impl Responder> {
  let account = repo
    .get_account(&confidential_account)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?;
  Ok(HttpResponse::Ok().json(account))
}

/// Create a new confidential account.
///
/// A confidential account is an Elgamal keypair.
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
#[post("/accounts/{confidential_account}/send")]
pub async fn request_sender_proof(
  confidential_account: web::Path<String>,
  req: web::Json<SenderProofRequest>,
  repo: Repository,
) -> Result<impl Responder> {
  // Get the account asset with account secret key.
  let account = repo
    .get_account_with_secret(&confidential_account)
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
#[post("/accounts/{confidential_account}/receiver_verify")]
pub async fn receiver_verify_request(
  confidential_account: web::Path<String>,
  req: web::Json<ReceiverVerifyRequest>,
  repo: Repository,
) -> Result<impl Responder> {
  // Get the account asset with account secret key.
  let account = repo
    .get_account_with_secret(&confidential_account)
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
#[post("/accounts/{confidential_account}/burn")]
pub async fn request_burn_proof(
  confidential_account: web::Path<String>,
  req: web::Json<BurnProofRequest>,
  repo: Repository,
) -> Result<impl Responder> {
  // Get the account asset with account secret key.
  let account = repo
    .get_account_with_secret(&confidential_account)
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
#[post("/accounts/{confidential_account}/decrypt")]
pub async fn decrypt_request(
  confidential_account: web::Path<String>,
  req: web::Json<AccountDecryptRequest>,
  repo: Repository,
) -> Result<impl Responder> {
  // Get the account asset with account secret key.
  let account = repo
    .get_account_with_secret(&confidential_account)
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
#[post("/accounts/{confidential_account}/auditor_verify")]
pub async fn auditor_verify_request(
  confidential_account: web::Path<String>,
  req: web::Json<AuditorVerifyRequest>,
  repo: Repository,
) -> Result<impl Responder> {
  // Get the account with secret key.
  let account = repo
    .get_account_with_secret(&confidential_account)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?;

  // Verify the sender's proof.
  let res = account.auditor_verify_proof(&req)?;
  Ok(HttpResponse::Ok().json(res))
}
