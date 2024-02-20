use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use sqlx::sqlite::SqlitePool;

use utoipa::OpenApi;
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

use polymesh_api::{client::IdentityId, Api};

use confidential_proof_api as proof_api;
use confidential_proof_api::{repo::SqliteConfidentialRepository, v1::*};
use confidential_proof_shared::*;
use confidential_rest_api::{repo::SqliteTransactionRepository, signing, v1::*};

pub fn v1_service(cfg: &mut web::ServiceConfig) {
  cfg.service(
    web::scope("/v1")
      //.configure(users::service)
      .configure(assets::service)
      .configure(accounts::service)
      .configure(signers::service)
      .configure(tx::service),
  );
}

async fn get_db_pool() -> anyhow::Result<SqlitePool> {
  let conn_str = std::env::var("DATABASE_URL")?;
  let pool = SqlitePool::connect(&conn_str).await?;
  sqlx::migrate!().run(&pool).await?;
  Ok(pool)
}

fn get_signing_manager(pool: &SqlitePool) -> anyhow::Result<signing::AppSigningManager> {
  let manager = std::env::var("SIGNING_MANAGER").ok();
  match manager.as_ref().map(|s| s.as_str()) {
    Some("DB" | "LOCAL") | None => Ok(signing::SqliteSigningManager::new_app_data(pool)),
    Some("VAULT") => {
      let base = std::env::var("VAULT_TRANSIT_URL")?;
      let token = std::env::var("VAULT_TOKEN")?;
      Ok(signing::VaultSigningManager::new_app_data(base, token)?)
    }
    Some(manager) => Err(anyhow::anyhow!("Unknown Signing Manager: {manager:?}")),
  }
}

async fn start_server() -> anyhow::Result<()> {
  // building address
  let port = std::env::var("PORT").unwrap_or("8080".to_string());
  let address = format!("0.0.0.0:{}", port);

  // Open database.
  let pool = get_db_pool().await?;
  // Repositories.
  let repo = SqliteConfidentialRepository::new_app_data(&pool);
  let tx_repo = SqliteTransactionRepository::new_app_data(&pool);
  log::info!("Repositories initialized");

  // Signing manager.
  let signing = get_signing_manager(&pool)?;

  let polymesh_url =
    std::env::var("POLYMESH_NODE_URL").unwrap_or("ws://localhost:9944/".to_string());
  let polymesh_api = web::Data::new(Api::new(&polymesh_url).await?);

  /*
  {
    use actix_web::rt;
    use confidential_rest_api::watcher;
    let repo = repo.clone();
    let tx_repo = tx_repo.clone();
    let api = (**polymesh_api).clone();
    log::info!("Starting chain watcher");
    rt::spawn(async move {
      if let Err(err) = watcher::start_chain_watcher(api, repo, tx_repo).await {
        log::error!("Chain watcher failed: {err:?}");
      }
    });
  }// */

  // starting the server
  log::info!("🚀🚀🚀 Starting Actix server at {}", address);

  #[derive(OpenApi)]
  #[openapi(
      paths(
        //users::get_all_users,
        //users::get_user,
        //users::create_user,
        signers::get_all_signers,
        signers::get_signer,
        signers::create_signer,
        signers::get_signer_identity,
        signers::get_signer_venues,
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
        tx::assets::tx_create_asset,
        tx::assets::tx_create_venue,
        tx::assets::get_asset_details,
        tx::assets::tx_allow_venues,
        tx::assets::tx_create_settlement,
        tx::assets::tx_execute_settlement,
        tx::accounts::tx_mediator_affirm_leg,
        tx::accounts::tx_affirm_transactions,
        tx::accounts::tx_init_account,
        tx::accounts::tx_account_did,
        tx::accounts::tx_apply_incoming_balances,
        tx::accounts::get_incoming_balances,
        tx::account_assets::tx_sender_affirm_leg,
        tx::account_assets::tx_receiver_affirm_leg,
        tx::account_assets::tx_apply_incoming,
        tx::account_assets::get_incoming_balance,
        tx::account_assets::tx_mint,
      ),
      components(
        schemas(
          User, CreateUser,
          SignerInfo, CreateSigner,
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
          DecryptedIncomingBalance,
          UpdateAccountAssetBalanceRequest,

          IdentityId,
          TransactionLegDetails,
          TransactionCreated,
          TransactionAffirmed,
          TransactionParty,
          ProcessedEvent,
          ProcessedEvents,
          TransactionArgs,
          TransactionResult,
          CreateConfidentialAsset,
          ConfidentialAssetDetails,
          ConfidentialSettlementLeg,
          CreateConfidentialSettlement,
          ExecuteConfidentialSettlement,
          AllowVenues,
          MintRequest,
          TransactionAssetAmount,
          AffirmTransactionLegRequest,
          AffirmTransactionLeg,
          AffirmTransactionRequest,
          AffirmTransactionsRequest,
          BalanceUpdated,
          BalanceUpdateAction,
          AccountAssetIncomingBalance,
          AccountAssetBalanceUpdated,
          AccountAssetBalancesUpdated,
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
          .app_data(tx_repo.clone())
          .app_data(signing.clone())
          .app_data(polymesh_api.clone())
          .configure(proof_api::health::service)
          .configure(v1_service),
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
