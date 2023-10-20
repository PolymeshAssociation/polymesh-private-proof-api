use actix_web::{get, post, web, HttpResponse, Responder, Result};

use polymesh_api::client::PairSigner;
use polymesh_api::types::{
  pallet_confidential_asset::TransactionId,
  polymesh_primitives::{
    asset::{AssetName, AssetType},
    settlement::VenueId,
  },
};
use polymesh_api::Api;

use confidential_proof_shared::{
  error::Error, AllowVenues, CreateAsset, CreateConfidentialAsset, CreateConfidentialSettlement,
  ExecuteConfidentialSettlement, SenderProofVerifyRequest,
  TransactionArgs, TransactionResult,
};

use crate::repo::Repository;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg
    .service(get_all_assets)
    .service(get_asset)
    .service(create_asset)
    .service(sender_proof_verify)
    .service(tx_create_asset)
    .service(tx_create_venue)
    .service(tx_allow_venues)
    .service(tx_create_settlement)
    .service(tx_execute_settlement);
}

/// Get all assets.
#[utoipa::path(
  responses(
    (status = 200, body = [Asset])
  )
)]
#[get("/assets")]
pub async fn get_all_assets(repo: web::Data<Repository>) -> Result<impl Responder> {
  let assets = repo.get_assets().await?;
  Ok(HttpResponse::Ok().json(assets))
}

/// Get an asset.
#[utoipa::path(
  responses(
    (status = 200, body = Asset)
  )
)]
#[get("/assets/{asset_id}")]
pub async fn get_asset(
  asset_id: web::Path<i64>,
  repo: web::Data<Repository>,
) -> Result<impl Responder> {
  Ok(match repo.get_asset(*asset_id).await? {
    Some(asset) => HttpResponse::Ok().json(asset),
    None => HttpResponse::NotFound().body("Not found"),
  })
}

/// Allow Venues.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/assets/{asset_id}/tx/allow_venues")]
pub async fn tx_allow_venues(
  asset_id: web::Path<i64>,
  req: web::Json<AllowVenues>,
  repo: web::Data<Repository>,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let ticker = repo
    .get_asset(*asset_id)
    .await?
    .ok_or_else(|| Error::not_found("Asset"))?
    .ticker()?;
  let mut signer = repo
    .get_signer_with_secret(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))
    .and_then(|signer| Ok(PairSigner::new(signer.keypair()?)))?;

  let venues = req.venues();
  let res = api
    .call()
    .confidential_asset()
    .allow_venues(ticker, venues)
    .map_err(|err| Error::from(err))?
    .submit_and_watch(&mut signer)
    .await
    .map_err(|err| Error::from(err))?;

  // Wait for transaction results.
  let res = TransactionResult::wait_for_results(res, req.finalize).await?;

  Ok(HttpResponse::Ok().json(res))
}

/// Create an asset.
#[utoipa::path(
  responses(
    (status = 200, body = Asset)
  )
)]
#[post("/assets")]
pub async fn create_asset(
  asset: web::Json<CreateAsset>,
  repo: web::Data<Repository>,
) -> Result<impl Responder> {
  let asset = repo.create_asset(&asset).await?;
  Ok(HttpResponse::Ok().json(asset))
}

/// Create confidential asset on-chain.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/assets/tx/create_asset")]
pub async fn tx_create_asset(
  req: web::Json<CreateConfidentialAsset>,
  repo: web::Data<Repository>,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let mut signer = repo
    .get_signer_with_secret(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))
    .and_then(|signer| Ok(PairSigner::new(signer.keypair()?)))?;

  let auditors = req.auditors()?;

  let ticker = req.ticker()?;

  let res = api
    .call()
    .confidential_asset()
    .create_confidential_asset(
      AssetName(req.name.as_bytes().into()),
      ticker,
      AssetType::EquityCommon,
      auditors,
    )
    .map_err(|err| Error::from(err))?
    .submit_and_watch(&mut signer)
    .await
    .map_err(|err| Error::from(err))?;

  // Wait for transaction results.
  let res = TransactionResult::wait_for_results(res, req.finalize).await?;

  Ok(HttpResponse::Ok().json(res))
}

/// Create confidential asset settlement.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/venues/{venue_id}/settlement/create")]
pub async fn tx_create_settlement(
  venue_id: web::Path<u64>,
  req: web::Json<CreateConfidentialSettlement>,
  repo: web::Data<Repository>,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let mut signer = repo
    .get_signer_with_secret(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))
    .and_then(|signer| Ok(PairSigner::new(signer.keypair()?)))?;

  let venue_id = VenueId(*venue_id);
  let memo = req.memo()?;
  let legs = req.legs()?;
  let res = api
    .call()
    .confidential_asset()
    .add_transaction(venue_id, legs, memo)
    .map_err(|err| Error::from(err))?
    .submit_and_watch(&mut signer)
    .await
    .map_err(|err| Error::from(err))?;

  // Wait for transaction results.
  let res = TransactionResult::wait_for_results(res, req.finalize).await?;

  Ok(HttpResponse::Ok().json(res))
}

/// Execute confidential asset settlement.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/settlements/{settlement_id}/execute")]
pub async fn tx_execute_settlement(
  transaction_id: web::Path<u64>,
  req: web::Json<ExecuteConfidentialSettlement>,
  repo: web::Data<Repository>,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let mut signer = repo
    .get_signer_with_secret(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))
    .and_then(|signer| Ok(PairSigner::new(signer.keypair()?)))?;

  let transaction_id = TransactionId(*transaction_id);
  let res = api
    .call()
    .confidential_asset()
    .execute_transaction(transaction_id, req.leg_count)
    .map_err(|err| Error::from(err))?
    .submit_and_watch(&mut signer)
    .await
    .map_err(|err| Error::from(err))?;

  // Wait for transaction results.
  let res = TransactionResult::wait_for_results(res, req.finalize).await?;

  Ok(HttpResponse::Ok().json(res))
}

/// Create Venue.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/assets/tx/create_venue")]
pub async fn tx_create_venue(
  req: web::Json<TransactionArgs>,
  repo: web::Data<Repository>,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let mut signer = repo
    .get_signer_with_secret(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))
    .and_then(|signer| Ok(PairSigner::new(signer.keypair()?)))?;

  let res = api
    .call()
    .confidential_asset()
    .create_venue()
    .map_err(|err| Error::from(err))?
    .submit_and_watch(&mut signer)
    .await
    .map_err(|err| Error::from(err))?;

  // Wait for transaction results.
  let res = TransactionResult::wait_for_results(res, req.finalize).await?;

  Ok(HttpResponse::Ok().json(res))
}

/// Verify a sender proof using only public information.
#[utoipa::path(
  responses(
    (status = 200, body = SenderProofVerifyResult)
  )
)]
#[post("/assets/sender_proof_verify")]
pub async fn sender_proof_verify(
  req: web::Json<SenderProofVerifyRequest>,
) -> Result<impl Responder> {
  // Verify the sender's proof.
  let res = req.verify_proof()?;
  Ok(HttpResponse::Ok().json(res))
}
