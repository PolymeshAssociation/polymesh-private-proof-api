use actix_web::{web, HttpResponse, Responder, Result};

use confidential_assets_api_shared::CreateAsset;

use crate::repo::ConfidentialRepository;

pub fn service<R: ConfidentialRepository>(cfg: &mut web::ServiceConfig) {
  cfg.service(
    web::scope("/assets")
      // GET
      .route("", web::get().to(get_all_assets::<R>))
      .route("/{asset_id}", web::get().to(get_asset::<R>))
      // POST
      .route("", web::post().to(create_asset::<R>)),
  );
}

/// Get all assets.
async fn get_all_assets<R: ConfidentialRepository>(repo: web::Data<R>) -> Result<impl Responder> {
  Ok(match repo.get_assets().await {
    Ok(assets) => HttpResponse::Ok().json(assets),
    Err(e) => HttpResponse::NotFound().body(format!("Internal server error: {:?}", e)),
  })
}

/// Get an asset.
async fn get_asset<R: ConfidentialRepository>(asset_id: web::Path<i64>, repo: web::Data<R>) -> HttpResponse {
  match repo.get_asset(*asset_id).await {
    Ok(asset) => HttpResponse::Ok().json(asset),
    Err(_) => HttpResponse::NotFound().body("Not found"),
  }
}

/// Create an asset.
async fn create_asset<R: ConfidentialRepository>(
  asset: web::Json<CreateAsset>,
  repo: web::Data<R>,
) -> HttpResponse {
  match repo.create_asset(&asset).await {
    Ok(asset) => HttpResponse::Ok().json(asset),
    Err(e) => HttpResponse::InternalServerError().body(format!("Internal server error: {:?}", e)),
  }
}
