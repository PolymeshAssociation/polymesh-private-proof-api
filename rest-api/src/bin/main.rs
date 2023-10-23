use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use sqlx::sqlite::SqlitePool;

use utoipa::OpenApi;
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

use polymesh_api::Api;

use confidential_proof_shared::*;
use confidential_rest_api as rest_api;
use confidential_rest_api::{repo, signing, v1::*};

async fn get_db_pool() -> anyhow::Result<SqlitePool> {
  let conn_str = std::env::var("DATABASE_URL")?;
  let pool = SqlitePool::connect(&conn_str).await?;
  sqlx::migrate!().run(&pool).await?;
  Ok(pool)
}

async fn start_server() -> anyhow::Result<()> {
  // building address
  let port = std::env::var("PORT").unwrap_or("8080".to_string());
  let address = format!("0.0.0.0:{}", port);

  // Open database.
  let pool = get_db_pool().await?;
  // Repository.
  let repo = web::Data::new(Box::new(repo::SqliteConfidentialRepository::new(pool.clone())));
  log::info!("Repository initialized");

  // Signing manager.
  let signing = web::Data::new(Box::new(signing::SqliteSigningManager::new(pool)));

  let polymesh_url = std::env::var("POLYMESH_URL").unwrap_or("ws://localhost:9944/".to_string());
  let polymesh_api = web::Data::new(Api::new(&polymesh_url).await?);

  // starting the server
  log::info!("🚀🚀🚀 Starting Actix server at {}", address);

  #[derive(OpenApi)]
  #[openapi(
      paths(
        users::get_all_users,
        users::get_user,
        users::create_user,
        signers::get_all_signers,
        signers::get_signer,
        signers::create_signer,
        assets::get_all_assets,
        assets::get_asset,
        assets::create_asset,
        assets::sender_proof_verify,
        assets::tx_create_asset,
        assets::tx_create_venue,
        assets::tx_allow_venues,
        assets::tx_create_settlement,
        assets::tx_execute_settlement,
        accounts::get_all_accounts,
        accounts::get_account,
        accounts::create_account,
        accounts::tx_add_mediator,
        accounts::tx_mediator_affirm_leg,
        accounts::auditor_verify_request,
        account_assets::get_all_account_assets,
        account_assets::get_account_asset,
        account_assets::create_account_asset,
        account_assets::tx_init_account,
        account_assets::tx_sender_affirm_leg,
        account_assets::tx_receiver_affirm_leg,
        account_assets::tx_apply_incoming,
        account_assets::tx_mint,
        account_assets::asset_issuer_mint,
        account_assets::request_sender_proof,
        account_assets::receiver_verify_request,
        account_assets::update_balance_request,
      ),
      components(
        schemas(
          User, CreateUser,
          Signer, CreateSigner,
          Asset, CreateAsset,
          Account,
          AccountAsset, CreateAccountAsset,
          AccountMintAsset,
          AccountAssetWithProof,
          PublicKey, SenderProof,
          AuditorVerifyRequest,
          ReceiverVerifyRequest,
          SenderProofRequest,
          SenderProofVerifyRequest,
          SenderProofVerifyResult,
          UpdateAccountAssetBalanceRequest,

          TransactionAffirmed,
          ProcessedEvent,
          ProcessedEvents,
          TransactionArgs,
          TransactionResult,
          CreateConfidentialAsset,
          AuditorRole,
          ConfidentialSettlementLeg,
          CreateConfidentialSettlement,
          ExecuteConfidentialSettlement,
          AllowVenues,
          MintRequest,
          AffirmTransactionLegRequest,
        ),
      ),
      servers(
        (url = "/api/v1/"),
      )
  )]
  struct ApiDoc;

  let openapi = ApiDoc::openapi();

  HttpServer::new(move || {
    // CORS
    let cors = Cors::permissive();

    App::new()
      .wrap(cors)
      .service(
        web::scope("/api")
          .app_data(repo.clone())
          .app_data(signing.clone())
          .app_data(polymesh_api.clone())
          .configure(rest_api::health::service)
          .configure(rest_api::v1::service),
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
