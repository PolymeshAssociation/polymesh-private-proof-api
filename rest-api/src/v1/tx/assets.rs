use actix_web::{get, post, web, HttpResponse, Responder, Result};
use uuid::Uuid;

use polymesh_api::types::{
  pallet_confidential_asset::TransactionId, polymesh_primitives::settlement::VenueId,
};
use polymesh_api::Api;

use confidential_proof_api::repo::Repository;
use confidential_proof_shared::{
  error::Error, scale_convert, AllowVenues, ConfidentialAssetDetails, CreateConfidentialAsset,
  CreateConfidentialSettlement, ExecuteConfidentialSettlement, TransactionArgs, TransactionResult,
};

use crate::signing::AppSigningManager;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg
    .service(tx_create_asset)
    .service(tx_create_venue)
    .service(tx_allow_venues)
    .service(get_asset_details)
    .service(tx_create_settlement)
    .service(tx_execute_settlement);
}

/// Get asset details.
#[utoipa::path(
  responses(
    (status = 200, body = ConfidentialAssetDetails)
  )
)]
#[get("/tx/assets/{asset_id}")]
pub async fn get_asset_details(
  asset_id: web::Path<Uuid>,
  _repo: Repository,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  // Get confidential asset details (name, ticker).
  let details = api
    .query()
    .confidential_asset()
    .details(*asset_id.as_bytes())
    .await
    .map_err(|err| Error::from(err))?
    .ok_or_else(|| Error::not_found("Confidential asset doesn't exist"))?;

  // Get and convert asset auditors.
  let asset_auditors = api
    .query()
    .confidential_asset()
    .asset_auditors(*asset_id.as_bytes())
    .await
    .map_err(|err| Error::from(err))?
    .ok_or_else(|| Error::not_found("Confidential asset doesn't exist"))?;
  let mediators = asset_auditors.mediators.iter().map(|d| d.clone()).collect();
  let auditors = asset_auditors
    .auditors
    .iter()
    .map(|k| scale_convert(k))
    .collect();

  let details = ConfidentialAssetDetails {
    total_supply: details.total_supply as u64,
    owner: details.owner_did,
    mediators,
    auditors,
  };
  Ok(HttpResponse::Ok().json(details))
}

/// Allow Venues.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/assets/{asset_id}/allow_venues")]
pub async fn tx_allow_venues(
  asset_id: web::Path<Uuid>,
  req: web::Json<AllowVenues>,
  _repo: Repository,
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let mut signer = signing
    .get_signer(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))?;

  let venues = req.venues();
  let res = api
    .call()
    .confidential_asset()
    .allow_venues(*asset_id.as_bytes(), venues)
    .map_err(|err| Error::from(err))?
    .submit_and_watch(&mut signer)
    .await
    .map_err(|err| Error::from(err))?;

  // Wait for transaction results.
  let res = TransactionResult::wait_for_results(res, req.finalize).await?;

  Ok(HttpResponse::Ok().json(res))
}

/// Create confidential asset on-chain.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/assets/create_asset")]
pub async fn tx_create_asset(
  req: web::Json<CreateConfidentialAsset>,
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let mut signer = signing
    .get_signer(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))?;

  let auditors = req.auditors()?;

  // TODO: Check if the mediators exist on-chain.

  let ticker = req.ticker()?;

  let res = api
    .call()
    .confidential_asset()
    .create_confidential_asset(ticker, vec![], auditors)
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
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let mut signer = signing
    .get_signer(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))?;

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
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let mut signer = signing
    .get_signer(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))?;

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
#[post("/tx/assets/create_venue")]
pub async fn tx_create_venue(
  req: web::Json<TransactionArgs>,
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let mut signer = signing
    .get_signer(&req.signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))?;

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
