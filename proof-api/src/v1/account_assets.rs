use actix_web::{get, post, web, HttpResponse, Responder, Result};

use confidential_proof_shared::{
  error::Error, AccountAssetWithProof, AccountMintAsset, CreateAccountAsset, ReceiverVerifyRequest,
  SenderProofRequest, SenderProofVerifyResult, UpdateAccountAssetBalanceRequest,
};

use crate::repo::Repository;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg
    .service(get_all_account_assets)
    .service(get_account_asset)
    .service(create_account_asset)
    .service(asset_issuer_mint)
    .service(request_sender_proof)
    .service(receiver_verify_request)
    .service(update_balance_request);
}

/// Get all assets for an account.
#[utoipa::path(
  responses(
    (status = 200, body = [AccountAsset])
  )
)]
#[get("/accounts/{account_id}/assets")]
pub async fn get_all_account_assets(
  account_id: web::Path<i64>,
  repo: web::Data<Repository>,
) -> Result<impl Responder> {
  let account_assets = repo.get_account_assets(*account_id).await?;
  Ok(HttpResponse::Ok().json(account_assets))
}

/// Get one asset for the account.
#[utoipa::path(
  responses(
    (status = 200, body = AccountAsset)
  )
)]
#[get("/accounts/{account_id}/assets/{asset_id}")]
pub async fn get_account_asset(
  path: web::Path<(i64, i64)>,
  repo: web::Data<Repository>,
) -> Result<impl Responder> {
  let (account_id, asset_id) = path.into_inner();
  let account_asset = repo
    .get_account_asset(account_id, asset_id)
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
#[post("/accounts/{account_id}/assets")]
pub async fn create_account_asset(
  account_id: web::Path<i64>,
  create_account_asset: web::Json<CreateAccountAsset>,
  repo: web::Data<Repository>,
) -> Result<impl Responder> {
  // Get the account's secret key.
  let account = repo
    .get_account_with_secret(*account_id)
    .await?
    .ok_or_else(|| Error::not_found("Account"))?;

  // Generate Account initialization proof.
  let init = account.init_balance(create_account_asset.asset_id);

  // Save initialize account balance.
  let account_asset = repo.create_account_asset(&init).await?;

  // Return account_asset.
  Ok(HttpResponse::Ok().json(account_asset))
}

/// Asset issuer updates their account balance when minting.
#[utoipa::path(
  responses(
    (status = 200, body = AccountAsset)
  )
)]
#[post("/accounts/{account_id}/assets/{asset_id}/mint")]
pub async fn asset_issuer_mint(
  path: web::Path<(i64, i64)>,
  account_mint_asset: web::Json<AccountMintAsset>,
  repo: web::Data<Repository>,
) -> Result<impl Responder> {
  let (account_id, asset_id) = path.into_inner();
  // Get the account asset with account secret key.
  let account_asset = repo
    .get_account_asset_with_secret(account_id, asset_id)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;

  // Mint asset.
  let update = account_asset.mint(account_mint_asset.amount)?;

  // Update account balance.
  let account_asset = repo
    .update_account_asset(&update)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;

  // Return account_asset.
  Ok(HttpResponse::Ok().json(account_asset))
}

/// Generate a sender proof.
#[utoipa::path(
  responses(
    (status = 200, body = AccountAssetWithProof)
  )
)]
#[post("/accounts/{account_id}/assets/{asset_id}/send")]
pub async fn request_sender_proof(
  path: web::Path<(i64, i64)>,
  req: web::Json<SenderProofRequest>,
  repo: web::Data<Repository>,
) -> Result<impl Responder> {
  let (account_id, asset_id) = path.into_inner();
  // Get the account asset with account secret key.
  let account_asset = repo
    .get_account_asset_with_secret(account_id, asset_id)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;

  // Generate sender proof.
  let (update, proof) = account_asset.create_send_proof(&req)?;

  // Update account balance.
  let account_asset = repo
    .update_account_asset(&update)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;

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
#[post("/accounts/{account_id}/assets/{asset_id}/receiver_verify")]
pub async fn receiver_verify_request(
  path: web::Path<(i64, i64)>,
  req: web::Json<ReceiverVerifyRequest>,
  repo: web::Data<Repository>,
) -> Result<impl Responder> {
  let (account_id, asset_id) = path.into_inner();
  // Get the account asset with account secret key.
  let account_asset = repo
    .get_account_asset_with_secret(account_id, asset_id)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;

  // Verify the sender's proof.
  let res = account_asset.receiver_verify_proof(&req);
  Ok(HttpResponse::Ok().json(SenderProofVerifyResult::from_result(res)))
}

/// Update an account's encrypted balance.
#[utoipa::path(
  responses(
    (status = 200, body = bool)
  )
)]
#[post("/accounts/{account_id}/assets/{asset_id}/update_balance")]
pub async fn update_balance_request(
  path: web::Path<(i64, i64)>,
  req: web::Json<UpdateAccountAssetBalanceRequest>,
  repo: web::Data<Repository>,
) -> Result<impl Responder> {
  let (account_id, asset_id) = path.into_inner();
  // Get the account asset with account secret key.
  let account_asset = repo
    .get_account_asset_with_secret(account_id, asset_id)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;

  // Prepare balance update.
  let update = account_asset.update_balance(&req)?;

  // Update account balance.
  let account_asset = repo
    .update_account_asset(&update)
    .await?
    .ok_or_else(|| Error::not_found("Account Asset"))?;

  // Return account_asset.
  Ok(HttpResponse::Ok().json(account_asset))
}
