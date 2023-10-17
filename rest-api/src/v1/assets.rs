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

use confidential_proof_shared::{CreateAsset, CreateConfidentialAsset};

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
  Ok(match repo.get_assets().await {
    Ok(assets) => HttpResponse::Ok().json(assets),
    Err(e) => HttpResponse::NotFound().body(format!("Internal server error: {:?}", e)),
  })
}

/// Get an asset.
#[utoipa::path(
  responses(
    (status = 200, body = Asset)
  )
)]
#[get("/assets/{asset_id}")]
pub async fn get_asset(asset_id: web::Path<i64>, repo: web::Data<Repository>) -> HttpResponse {
  match repo.get_asset(*asset_id).await {
    Ok(asset) => HttpResponse::Ok().json(asset),
    Err(_) => HttpResponse::NotFound().body("Not found"),
  }
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
) -> HttpResponse {
  match repo.create_asset(&asset).await {
    Ok(asset) => HttpResponse::Ok().json(asset),
    Err(e) => HttpResponse::InternalServerError().body(format!("Internal server error: {:?}", e)),
  }
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
) -> HttpResponse {
  let mut signer = match repo.get_signer_with_secret(&asset.signer).await {
    Ok(signer) => PairSigner::new(signer.keypair().expect("Keypair from db.")),
    Err(e) => return HttpResponse::InternalServerError().body(format!("Internal server error: {:?}", e)),
  };

  let auditors = match asset.auditors() {
    Ok(auditors) => auditors,
    Err(e) => return HttpResponse::InternalServerError().body(format!("Internal server error: {:?}", e)),
  };

  let ticker = match asset.ticker() {
    Ok(ticker) => ticker,
    Err(e) => return HttpResponse::InternalServerError().body(format!("Internal server error: {:?}", e)),
  };

  let res = api.call()
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

  HttpResponse::Ok().json(true)
}
