use actix_web::{post, web, HttpResponse, Responder, Result};

use polymesh_api::client::PairSigner;
use polymesh_api::types::{
  pallet_confidential_asset::TransactionId,
  polymesh_primitives::{
    asset::{AssetName, AssetType},
    settlement::VenueId,
  },
};
use polymesh_api::Api;

use confidential_proof_api::repo::Repository;
use confidential_proof_shared::{
  error::Error, AllowVenues, CreateConfidentialAsset, CreateConfidentialSettlement,
  ExecuteConfidentialSettlement, TransactionArgs, TransactionResult,
};

use crate::signing::SigningManager;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg
    .service(tx_create_asset)
    .service(tx_create_venue)
    .service(tx_allow_venues)
    .service(tx_create_settlement)
    .service(tx_execute_settlement);
}

/// Allow Venues.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/assets/{asset_id}/allow_venues")]
pub async fn tx_allow_venues(
  asset_id: web::Path<i64>,
  req: web::Json<AllowVenues>,
  repo: web::Data<Repository>,
  signing: web::Data<SigningManager>,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let ticker = repo
    .get_asset(*asset_id)
    .await?
    .ok_or_else(|| Error::not_found("Asset"))?
    .ticker()?;
  let mut signer = signing
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

/// Create confidential asset on-chain.
#[utoipa::path(
  responses(
    (status = 200, body = TransactionResult)
  )
)]
#[post("/tx/assets/create_asset")]
pub async fn tx_create_asset(
  req: web::Json<CreateConfidentialAsset>,
  signing: web::Data<SigningManager>,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let mut signer = signing
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
  signing: web::Data<SigningManager>,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let mut signer = signing
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
  signing: web::Data<SigningManager>,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let mut signer = signing
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
#[post("/tx/assets/create_venue")]
pub async fn tx_create_venue(
  req: web::Json<TransactionArgs>,
  signing: web::Data<SigningManager>,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let mut signer = signing
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
