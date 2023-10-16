use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use sqlx::sqlite::SqlitePool;

use utoipa::OpenApi;
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

use confidential_rest_api as rest_api;
use confidential_rest_api::{repo, v1::*};
use confidential_proof_shared::*;

async fn get_repo() -> Result<repo::Repository, sqlx::Error> {
  let conn_str =
    std::env::var("DATABASE_URL").map_err(|e| sqlx::Error::Configuration(Box::new(e)))?;
  let pool = SqlitePool::connect(&conn_str).await?;
  sqlx::migrate!().run(&pool).await?;
  Ok(Box::new(repo::SqliteConfidentialRepository::new(pool)))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  if std::env::var_os("RUST_LOG").is_none() {
    std::env::set_var("RUST_LOG", "actix_web=info");
  }
  // env vars
  dotenv::dotenv().ok();
  env_logger::init();

  // building address
  let port = std::env::var("PORT").unwrap_or("8080".to_string());
  let address = format!("0.0.0.0:{}", port);

  // repository
  let repo = get_repo().await.expect("Couldn't get the repository");
  let repo = web::Data::new(repo);
  log::info!("Repository initialized");

  // starting the server
  log::info!("🚀🚀🚀 Starting Actix server at {}", address);

  #[derive(OpenApi)]
  #[openapi(
      paths(
        users::get_all_users,
        users::get_user,
        users::create_user,
        assets::get_all_assets,
        assets::get_asset,
        assets::create_asset,
        accounts::get_all_accounts,
        accounts::get_account,
        accounts::create_account,
        accounts::auditor_verify_request,
        account_assets::get_all_account_assets,
        account_assets::get_account_asset,
        account_assets::create_account_asset,
        account_assets::asset_issuer_mint,
        account_assets::request_sender_proof,
        account_assets::receiver_verify_request,
        account_assets::update_balance_request,
      ),
      components(
        schemas(
          User, CreateUser,
          Asset, CreateAsset,
          Account, CreateAccount,
          AccountAsset, CreateAccountAsset,
          AccountMintAsset,
          AccountAssetWithTx,
          PublicKey, SenderProof,
          AuditorVerifyRequest,
          ReceiverVerifyRequest,
          SenderProofRequest,
          UpdateAccountAssetBalanceRequest,
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
  .unwrap_or_else(|err| {
    panic!(
      "🔥🔥🔥 Couldn't start the server in port {}: {:?}",
      port, err
    )
  })
  .run()
  .await
}
