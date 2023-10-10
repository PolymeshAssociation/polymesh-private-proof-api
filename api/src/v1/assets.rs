use actix_web::{web, HttpResponse, Responder, Result};

use confidential_assets_api_shared::CreateAsset;

use crate::repo::MercatRepository;

pub fn service<R: MercatRepository>(cfg: &mut web::ServiceConfig) {
  cfg.service(
    web::scope("/assets")
      // GET
      .route("", web::get().to(get_all::<R>))
      .route("/{asset_id}", web::get().to(get::<R>))
      // POST
      .route("", web::post().to(post::<R>)),
  );
}

async fn get_all<R: MercatRepository>(repo: web::Data<R>) -> Result<impl Responder> {
  Ok(match repo.get_assets().await {
    Ok(assets) => HttpResponse::Ok().json(assets),
    Err(e) => HttpResponse::NotFound().body(format!("Internal server error: {:?}", e)),
  })
}

async fn get<R: MercatRepository>(asset_id: web::Path<i64>, repo: web::Data<R>) -> HttpResponse {
  match repo.get_asset(*asset_id).await {
    Ok(asset) => HttpResponse::Ok().json(asset),
    Err(_) => HttpResponse::NotFound().body("Not found"),
  }
}

async fn post<R: MercatRepository>(
  asset: web::Json<CreateAsset>,
  repo: web::Data<R>,
) -> HttpResponse {
  match repo.create_asset(&asset).await {
    Ok(asset) => HttpResponse::Ok().json(asset),
    Err(e) => HttpResponse::InternalServerError().body(format!("Internal server error: {:?}", e)),
  }
}
