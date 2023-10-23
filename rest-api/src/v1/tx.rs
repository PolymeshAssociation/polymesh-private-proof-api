use actix_web::web;

pub mod account_assets;
pub mod accounts;
pub mod assets;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg.configure(assets::service).configure(accounts::service);
}
