use actix_web::{get, post, web, HttpResponse, Responder, Result};
use uuid::Uuid;

use confidential_proof_shared::{
  error::Error, AccountAssetDecryptRequest, AccountAssetWithProof, CreateAccountAsset,
  ReceiverVerifyRequest, SenderProofRequest, UpdateAccountAssetBalanceRequest,
};

use crate::repo::Repository;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg
    .service(get_all_account_assets)
    .service(get_account_asset)
    .service(create_account_asset)
    .service(request_sender_proof)
    .service(receiver_verify_request)
    .service(decrypt_request)
    .service(update_balance_request);
}

/// Get all assets for an account.
#[utoipa::path(
  responses(
    (status = 200, body = [AccountAsset])
  )
)]
#[get("/accounts/{public_key}/assets")]
pub async fn get_all_account_assets(
  public_key: web::Path<String>,
  repo: Repository,
) -> Result<impl Responder> {
  let account_assets = repo.get_account_assets(&public_key).await?;
  Ok(HttpResponse::Ok().json(account_assets))
}

/// Get one asset for the account.
#[utoipa::path(
  responses(
    (status = 200, body = AccountAsset)
  )
)]
#[get("/accounts/{public_key}/assets/{asset_id}")]
pub async fn get_account_asset(
  path: web::Path<(String, Uuid)>,
  repo: Repository,
) -> Result<impl Responder> {
  let (public_key, asset_id) = path.into_inner();
  let account_asset = repo
    .get_account_asset(&public_key, asset_id)
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
#[post("/accounts/{public_key}/assets")]
pub async fn create_account_asset(
  public_key: web::Path<String>,
  create_account_asset: web::Json<CreateAccountAsset>,
  repo: Repository,
) -> Result<impl Responder> {
  // Get the account's secret key.
  let account = repo
    .get_account_with_secret(&public_key)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?;
  let asset = repo
    .get_asset(create_account_asset.asset_id)
    .await?
    .ok_or_else(|| Error::not_found("Asset"))?;

  // Generate Account initialization proof.
  let init = account.init_balance(asset.asset_id);

  // Save initialize account balance.
  let account_asset = repo.create_account_asset(&init).await?;

  // Return account_asset.
  Ok(HttpResponse::Ok().json(account_asset))
}

/// Generate a sender proof.
#[utoipa::path(
  responses(
    (status = 200, body = AccountAssetWithProof)
  )
)]
#[post("/accounts/{public_key}/assets/{asset_id}/send")]
pub async fn request_sender_proof(
  path: web::Path<(String, Uuid)>,
  req: web::Json<SenderProofRequest>,
  repo: Repository,
) -> Result<impl Responder> {
  let (public_key, asset_id) = path.into_inner();
  // Get the account asset with account secret key.
  let account_asset = repo
    .get_account_asset_with_secret(&public_key, asset_id)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;

  let enc_balance = req.encrypted_balance()?;
  let receiver = req.receiver()?;
  let auditors = req.auditors()?;
  let amount = req.amount;

  // Generate sender proof.
  let (update, proof) = account_asset.create_send_proof(enc_balance, receiver, auditors, amount)?;

  // Update account balance.
  let account_asset = repo.update_account_asset(&update).await?;

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
#[post("/accounts/{public_key}/assets/{asset_id}/receiver_verify")]
pub async fn receiver_verify_request(
  path: web::Path<(String, Uuid)>,
  req: web::Json<ReceiverVerifyRequest>,
  repo: Repository,
) -> Result<impl Responder> {
  let (public_key, asset_id) = path.into_inner();
  // Get the account asset with account secret key.
  let account_asset = repo
    .get_account_asset_with_secret(&public_key, asset_id)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;

  // Verify the sender's proof.
  let res = account_asset.receiver_verify_proof(&req)?;
  Ok(HttpResponse::Ok().json(res))
}

/// Decrypt a `CipherText` value.
#[utoipa::path(
  responses(
    (status = 200, body = DecryptedResponse)
  )
)]
#[post("/accounts/{public_key}/assets/{asset_id}/decrypt")]
pub async fn decrypt_request(
  path: web::Path<(String, Uuid)>,
  req: web::Json<AccountAssetDecryptRequest>,
  repo: Repository,
) -> Result<impl Responder> {
  let (public_key, asset_id) = path.into_inner();
  // Get the account asset with account secret key.
  let account_asset = repo
    .get_account_asset_with_secret(&public_key, asset_id)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;

  // Decrypt the value.
  let resp = account_asset.decrypt_request(&req)?;

  // Return the decrypted value.
  Ok(HttpResponse::Ok().json(resp))
}

/// Update an account's encrypted balance.
#[utoipa::path(
  responses(
    (status = 200, body = AccountAsset)
  )
)]
#[post("/accounts/{public_key}/assets/{asset_id}/update_balance")]
pub async fn update_balance_request(
  path: web::Path<(String, Uuid)>,
  req: web::Json<UpdateAccountAssetBalanceRequest>,
  repo: Repository,
) -> Result<impl Responder> {
  let (public_key, asset_id) = path.into_inner();
  // Get the account asset with account secret key.
  let account_asset = repo
    .get_account_asset_with_secret(&public_key, asset_id)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;

  // Prepare balance update.
  let update = account_asset.update_balance(&req)?;

  // Update account balance.
  let account_asset = repo.update_account_asset(&update).await?;

  // Return account_asset.
  Ok(HttpResponse::Ok().json(account_asset))
}
