use actix_web::web;

mod account_assets;
mod accounts;
mod assets;
mod users;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg.service(
    web::scope("/v1")
      .configure(users::service)
      .configure(assets::service)
      .configure(accounts::service),
  );
}
