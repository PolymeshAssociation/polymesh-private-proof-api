use actix_web::{get, post, rt::pin, web, HttpResponse, Responder, Result};
use futures_util::StreamExt;

use polymesh-private-proof-shared::{error::Error, CreateSigner};

use polymesh_api::Api;
use polymesh_api::{
  client::basic_types::IdentityId, types::polymesh_primitives::secondary_key::KeyRecord,
};

use crate::signing::AppSigningManager;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg
    .service(get_all_signers)
    .service(get_signer)
    .service(create_signer)
    .service(get_signer_identity)
    .service(get_signer_venues);
}

/// Get all signers.
#[utoipa::path(
  responses(
    (status = 200, body = [SignerInfo])
  )
)]
#[get("/signers")]
pub async fn get_all_signers(signing: AppSigningManager) -> Result<impl Responder> {
  let signers = signing.get_signers().await?;
  Ok(HttpResponse::Ok().json(signers))
}

/// Get one signer.
#[utoipa::path(
  responses(
    (status = 200, body = SignerInfo)
  )
)]
#[get("/signers/{signer}")]
pub async fn get_signer(
  signer: web::Path<String>,
  signing: AppSigningManager,
) -> Result<impl Responder> {
  Ok(match signing.get_signer_info(&signer).await? {
    Some(signer) => HttpResponse::Ok().json(signer),
    None => HttpResponse::NotFound().body("Not found"),
  })
}

/// Get signer's identity id (DID).
pub async fn get_signer_did(
  signer: &str,
  signing: AppSigningManager,
  api: &Api,
) -> Result<Option<IdentityId>> {
  let signer = signing
    .get_signer_info(signer)
    .await?
    .ok_or_else(|| Error::not_found("Signer"))?;
  let account_id = signer.account_id()?;
  let did = api
    .query()
    .identity()
    .key_records(account_id)
    .await
    .map_err(|err| Error::from(err))?
    .and_then(|key| match key {
      KeyRecord::PrimaryKey(did) | KeyRecord::SecondaryKey(did, _) => Some(did),
      _ => None,
    });
  Ok(did)
}

/// Get signer's identity id.
#[utoipa::path(
  responses(
    (status = 200, body = Option<String>)
  )
)]
#[get("/signers/{signer}/identity")]
pub async fn get_signer_identity(
  signer: web::Path<String>,
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let did = get_signer_did(&signer, signing, &api)
    .await?
    .map(|did| format!("{did:?}"));
  Ok(HttpResponse::Ok().json(did))
}

/// Get signer's confidential venues.
#[utoipa::path(
  responses(
    (status = 200, body = Option<Vec<u64>>)
  )
)]
#[get("/signers/{signer}/venues")]
pub async fn get_signer_venues(
  signer: web::Path<String>,
  signing: AppSigningManager,
  api: web::Data<Api>,
) -> Result<impl Responder> {
  let did = get_signer_did(&signer, signing, &api).await?;
  let venues = match did {
    Some(did) => {
      let mut venues = Vec::new();
      let ids = api
        .paged_query()
        .confidential_asset()
        .identity_venues(did)
        .keys();
      pin!(ids);
      while let Some(venue_id) = ids.next().await {
        if let Ok(venue_id) = venue_id {
          venues.push(venue_id.0);
        }
      }
      Some(venues)
    }
    None => None,
  };

  Ok(HttpResponse::Ok().json(venues))
}

/// Create a new signer.
#[utoipa::path(
  responses(
    (status = 200, body = SignerInfo)
  )
)]
#[post("/signers")]
pub async fn create_signer(
  signer: web::Json<CreateSigner>,
  signing: AppSigningManager,
) -> Result<impl Responder> {
  let signer = signing.create_signer(&signer).await?;
  Ok(HttpResponse::Ok().json(signer))
}
