use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use sqlx::sqlite::SqlitePool;

use utoipa::OpenApi;
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

use polymesh_private_proof_api as proof_api;
use polymesh_private_proof_api::{repo, v1::*};
use polymesh_private_proof_shared::*;

async fn get_db_pool() -> anyhow::Result<SqlitePool> {
  let conn_str = std::env::var("DATABASE_URL")?;
  let pool = SqlitePool::connect(&conn_str).await?;
  sqlx::migrate!().run(&pool).await?;
  Ok(pool)
}

async fn start_server() -> anyhow::Result<()> {
  // building address
  let port = std::env::var("PORT").unwrap_or("8080".to_string());
  let bind_address = std::env::var("BIND_ADDRESS").unwrap_or("0.0.0.0".to_string());
  let address = format!("{}:{}", bind_address, port);

  // Open database.
  let pool = get_db_pool().await?;
  // Repository.
  let repo = repo::SqliteConfidentialRepository::new_app_data(&pool);
  log::info!("Repository initialized");

  // starting the server
  log::info!("🚀🚀🚀 Starting Actix server at {}", address);

  #[derive(OpenApi)]
  #[cfg_attr(not(feature = "track_balances"),
    openapi(
        paths(
          //users::get_all_users,
          //users::get_user,
          //users::create_user,
          accounts::get_all_accounts,
          accounts::get_account,
          accounts::create_account,
          accounts::auditor_verify_request,
          accounts::request_sender_proof,
          accounts::request_burn_proof,
          accounts::receiver_verify_request,
          accounts::decrypt_request,
        ),
        components(
          schemas(
            User, CreateUser,
            Account,
            PublicKey, BurnProof, SenderProof, TransferProofs,
            AuditorVerifyRequest,
            ReceiverVerifyRequest,
            BurnProofRequest,
            SenderProofRequest,
            SenderProofVerifyRequest,
            SenderProofVerifyResult,
            AccountDecryptRequest,
            DecryptedResponse,
          ),
        ),
        servers(
          (url = "/api/v1/"),
        )
    )
  )]
  #[cfg_attr(feature = "track_balances",
    openapi(
        paths(
          //users::get_all_users,
          //users::get_user,
          //users::create_user,
          assets::get_all_assets,
          assets::get_asset,
          assets::create_asset,
          assets::sender_proof_verify,
          accounts::get_all_accounts,
          accounts::get_account,
          accounts::create_account,
          accounts::auditor_verify_request,
          accounts::request_sender_proof,
          accounts::request_burn_proof,
          accounts::receiver_verify_request,
          accounts::decrypt_request,
          account_assets::get_all_account_assets,
          account_assets::get_account_asset,
          account_assets::create_account_asset,
          account_assets::request_sender_proof,
          account_assets::request_burn_proof,
          account_assets::receiver_verify_request,
          account_assets::update_balance_request,
          account_assets::decrypt_request,
        ),
        components(
          schemas(
            User, CreateUser,
            Asset, AddAsset,
            Account,
            AccountAsset, CreateAccountAsset,
            AccountAssetWithProof,
            PublicKey, BurnProof, SenderProof, TransferProofs,
            AuditorVerifyRequest,
            ReceiverVerifyRequest,
            BurnProofRequest,
            SenderProofRequest,
            SenderProofVerifyRequest,
            SenderProofVerifyResult,
            AccountDecryptRequest,
            DecryptedResponse,
            UpdateAccountAssetBalanceRequest,
          ),
        ),
        servers(
          (url = "/api/v1/"),
        )
    )
  )]
  struct ApiDoc;

  let openapi = ApiDoc::openapi();

  HttpServer::new(move || {
    // CORS
    let cors = Cors::permissive();

    App::new()
      .wrap(cors)
      .service(web::redirect("/", "/swagger-ui/"))
      .service(
        web::scope("/api")
          .app_data(repo.clone())
          .configure(proof_api::health::service)
          .configure(proof_api::v1::service),
      )
      .service(Redoc::with_url("/redoc", openapi.clone()))
      .service(SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", openapi.clone()))
      // There is no need to create RapiDoc::with_openapi because the OpenApi is served
      // via SwaggerUi instead we only make rapidoc to point to the existing doc.
      .service(RapiDoc::new("/api-docs/openapi.json").path("/rapidoc"))
      .wrap(Logger::default())
  })
  .bind(&address)
  .map_err(|err| {
    log::error!("🔥🔥🔥 Couldn't start the server on address & port {address}: {err:?}",);
    err
  })?
  .run()
  .await?;
  Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  if std::env::var_os("RUST_LOG").is_none() {
    std::env::set_var("RUST_LOG", "actix_web=info");
  }
  // env vars
  dotenv::dotenv().ok();
  env_logger::init();

  if let Err(err) = start_server().await {
    log::error!("Failed to start server: {err:?}");
    return Err(std::io::Error::new(std::io::ErrorKind::Other, err));
  }
  Ok(())
}
