use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use sqlx::sqlite::SqlitePool;

use utoipa::{openapi::OpenApiBuilder, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

use confidential_assets_api as api;
use confidential_assets_api::repo::Repository;
use confidential_assets_api_shared::*;

async fn get_repo() -> Result<Repository, sqlx::Error> {
  let conn_str =
    std::env::var("DATABASE_URL").map_err(|e| sqlx::Error::Configuration(Box::new(e)))?;
  let pool = SqlitePool::connect(&conn_str).await?;
  sqlx::migrate!().run(&pool).await?;
  Ok(Box::new(api::repo::SqliteConfidentialRepository::new(pool)))
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
  let address = format!("127.0.0.1:{}", port);

  // repository
  let repo = get_repo().await.expect("Couldn't get the repository");
  let repo = web::Data::new(repo);
  log::info!("Repository initialized");

  // starting the server
  log::info!("🚀🚀🚀 Starting Actix server at {}", address);

  #[derive(OpenApi)]
  #[openapi(
      paths(
        api::v1::users::get_all_users,
        api::v1::users::get_user,
        api::v1::users::create_user,
        api::v1::assets::get_all_assets,
        api::v1::assets::get_asset,
        api::v1::assets::create_asset,
        api::v1::accounts::get_all_accounts,
        api::v1::accounts::get_account,
        api::v1::accounts::create_account,
        api::v1::accounts::auditor_verify_request,
        api::v1::account_assets::get_all_account_assets,
        api::v1::account_assets::get_account_asset,
        api::v1::account_assets::create_account_asset,
        api::v1::account_assets::asset_issuer_mint,
        api::v1::account_assets::request_sender_proof,
        api::v1::account_assets::receiver_verify_request,
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
          SenderProofRequest,
          ReceiverVerifyRequest,
          AuditorVerifyRequest,
        ),
      ),
      servers(
        (url = "/api/v1/"),
      )
  )]
  struct ApiDoc;

  let builder: OpenApiBuilder = ApiDoc::openapi().into();

  let openapi = builder
    //.paths(api::v1::users::__path_get_all_users)
    .build();

  HttpServer::new(move || {
    // CORS
    let cors = Cors::permissive();

    App::new()
      .wrap(cors)
      .service(
        web::scope("/api")
          .app_data(repo.clone())
          .configure(api::health::service)
          .configure(api::v1::service),
      )
      .service(SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", openapi.clone()))
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
