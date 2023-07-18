use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use sqlx::sqlite::SqlitePool;

use mercat_api as api;

type BackendRepository = api::repo::SqliteMercatRepository;

async fn get_repo() -> Result<BackendRepository, sqlx::Error> {
    let conn_str =
        std::env::var("DATABASE_URL").map_err(|e| sqlx::Error::Configuration(Box::new(e)))?;
    let pool = SqlitePool::connect(&conn_str).await?;
    sqlx::migrate!().run(&pool).await?;
    Ok(api::repo::SqliteMercatRepository::new(pool))
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

    HttpServer::new(move || {
        // CORS
        let cors = Cors::permissive();

        App::new()
            .wrap(cors)
            .service(
                web::scope("/api")
                    .app_data(repo.clone())
                    .configure(api::health::service)
                    .configure(api::v1::service::<BackendRepository>)
            )
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