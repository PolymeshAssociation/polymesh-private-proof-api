use actix_web::{get, post, web, HttpResponse, Responder, Result};

use confidential_proof_api_shared::CreateAsset;

use crate::repo::Repository;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg
    .service(get_all_assets)
    .service(get_asset)
    .service(create_asset);
}

/// Get all assets.
#[utoipa::path(
  responses(
    (status = 200, description = "List all assets", body = [Asset])
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
    (status = 200, description = "Get an asset", body = Asset)
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
    (status = 200, description = "Create an asset", body = Asset)
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
