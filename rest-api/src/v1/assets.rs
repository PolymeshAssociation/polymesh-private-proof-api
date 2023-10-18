use actix_web::{get, post, web, HttpResponse, Responder, Result};

use polymesh_api::types::{
    pallet_confidential_asset::{
        ConfidentialAuditors,
    },
    polymesh_primitives::{
        asset::{AssetName, AssetType},
    },
};
use polymesh_api::client::PairSigner;
use polymesh_api::Api;

use confidential_proof_shared::{error::Error, CreateAsset, CreateConfidentialAsset};

use crate::repo::Repository;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg
    .service(get_all_assets)
    .service(get_asset)
    .service(create_asset)
    .service(tx_create_asset);
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
pub async fn get_asset(asset_id: web::Path<i64>, repo: web::Data<Repository>) -> Result<impl Responder> {
  Ok(match repo.get_asset(*asset_id).await? {
    Some(asset) => HttpResponse::Ok().json(asset),
    None => HttpResponse::NotFound().body("Not found"),
  })
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
    (status = 200, body = Asset)
  )
)]
#[post("/assets/tx/create")]
pub async fn tx_create_asset(
  asset: web::Json<CreateConfidentialAsset>,
  repo: web::Data<Repository>,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let mut signer = repo.get_signer_with_secret(&asset.signer).await?
    .ok_or_else(|| Error::not_found("Signer"))
    .and_then(|signer| Ok(PairSigner::new(signer.keypair()?)))?;

  let auditors = asset.auditors()?;

  let ticker = asset.ticker()?;

  let _res = api.call()
    .confidential_asset()
    .create_confidential_asset(
      AssetName(asset.name.as_bytes().into()),
      ticker,
      AssetType::EquityCommon,
      ConfidentialAuditors { auditors },
    )
    .expect("tx")
    .submit_and_watch(&mut signer)
    .await;

  Ok(HttpResponse::Ok().json(true))
}
