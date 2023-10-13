use actix_web::web;

pub mod account_assets;
pub mod accounts;
pub mod assets;
pub mod users;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg.service(
    web::scope("/v1")
      .configure(users::service)
      .configure(assets::service)
      .configure(accounts::service),
  );
}
