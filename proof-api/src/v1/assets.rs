use actix_web::{get, post, web, HttpResponse, Responder, Result};

use confidential_proof_shared::{CreateAsset, SenderProofVerifyRequest, SenderProofVerifyResult};

use crate::repo::Repository;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg
    .service(get_all_assets)
    .service(get_asset)
    .service(create_asset)
    .service(sender_proof_verify);
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
  let res = req.verify_proof();
  Ok(HttpResponse::Ok().json(SenderProofVerifyResult::from_result(res)))
}
