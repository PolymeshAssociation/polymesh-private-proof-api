use actix_web::{get, post, web, HttpResponse, Responder, Result};

use confidential_proof_shared::CreateSigner;

use crate::repo::Repository;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg
    .service(get_all_signers)
    .service(get_signer)
    .service(create_signer);
}

/// Get all signers.
#[utoipa::path(
  responses(
    (status = 200, body = [Signer])
  )
)]
#[get("/signers")]
pub async fn get_all_signers(repo: web::Data<Repository>) -> Result<impl Responder> {
  Ok(match repo.get_signers().await {
    Ok(signers) => HttpResponse::Ok().json(signers),
    Err(e) => HttpResponse::NotFound().body(format!("Internal server error: {:?}", e)),
  })
}

/// Get one signer.
#[utoipa::path(
  responses(
    (status = 200, body = Signer)
  )
)]
#[get("/signers/{signer}")]
pub async fn get_signer(signer: web::Path<String>, repo: web::Data<Repository>) -> HttpResponse {
  match repo.get_signer(&signer).await {
    Ok(signer) => HttpResponse::Ok().json(signer),
    Err(_) => HttpResponse::NotFound().body("Not found"),
  }
}

/// Create a new signer.
#[utoipa::path(
  responses(
    (status = 200, body = Signer)
  )
)]
#[post("/signers")]
pub async fn create_signer(signer: web::Json<CreateSigner>, repo: web::Data<Repository>) -> HttpResponse {
  let signer = match signer.as_signer_with_secret() {
    Ok(signer) => signer,
    Err(e) => return HttpResponse::InternalServerError().body(format!("Internal server error: {:?}", e)),
  };
  match repo.create_signer(&signer).await {
    Ok(signer) => HttpResponse::Ok().json(signer),
    Err(e) => HttpResponse::InternalServerError().body(format!("Internal server error: {:?}", e)),
  }
}
