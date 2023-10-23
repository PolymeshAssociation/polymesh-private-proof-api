use actix_web::{get, post, web, HttpResponse, Responder, Result};

use confidential_proof_shared::CreateSigner;

use crate::signing::SigningManager;

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
pub async fn get_all_signers(signing: web::Data<SigningManager>) -> Result<impl Responder> {
  let signers = signing.get_signers().await?;
  Ok(HttpResponse::Ok().json(signers))
}

/// Get one signer.
#[utoipa::path(
  responses(
    (status = 200, body = Signer)
  )
)]
#[get("/signers/{signer}")]
pub async fn get_signer(
  signer: web::Path<String>,
  signing: web::Data<SigningManager>,
) -> Result<impl Responder> {
  Ok(match signing.get_signer(&signer).await? {
    Some(signer) => HttpResponse::Ok().json(signer),
    None => HttpResponse::NotFound().body("Not found"),
  })
}

/// Create a new signer.
#[utoipa::path(
  responses(
    (status = 200, body = Signer)
  )
)]
#[post("/signers")]
pub async fn create_signer(
  signer: web::Json<CreateSigner>,
  signing: web::Data<SigningManager>,
) -> Result<impl Responder> {
  let signer = signer.as_signer_with_secret()?;
  let signer = signing.create_signer(&signer).await?;
  Ok(HttpResponse::Ok().json(signer))
}
