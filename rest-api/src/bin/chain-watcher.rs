use sqlx::sqlite::SqlitePool;

use polymesh_api::Api;

use confidential_proof_api::repo::SqliteConfidentialRepository;

use confidential_rest_api::repo::SqliteTransactionRepository;
use confidential_rest_api::watcher::*;

async fn get_db_pool() -> anyhow::Result<SqlitePool> {
  let conn_str = std::env::var("DATABASE_URL")?;
  let pool = SqlitePool::connect(&conn_str).await?;
  sqlx::migrate!().run(&pool).await?;
  Ok(pool)
}

async fn start_watcher() -> anyhow::Result<()> {
  // Open database.
  let pool = get_db_pool().await?;
  // Repositories.
  let repo = SqliteConfidentialRepository::new_app_data(&pool);
  let tx_repo = SqliteTransactionRepository::new_app_data(&pool);
  log::info!("Repositories initialized");

  let polymesh_url =
    std::env::var("POLYMESH_NODE_URL").unwrap_or("ws://localhost:9944/".to_string());
  let api = Api::new(&polymesh_url).await?;

  // starting the server
  log::info!("🚀🚀🚀 Starting chain watcher");

  start_chain_watcher(api, repo, tx_repo).await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  // env vars
  dotenv::dotenv().ok();
  env_logger::init();

  if let Err(err) = start_watcher().await {
    log::error!("Failed to start server: {err:?}");
    return Err(std::io::Error::new(std::io::ErrorKind::Other, err));
  }
  Ok(())
}
