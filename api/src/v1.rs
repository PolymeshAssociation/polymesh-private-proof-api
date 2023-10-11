use actix_web::web;

use crate::repo::ConfidentialRepository;

mod account_assets;
mod accounts;
mod assets;
mod users;

pub fn service<R: ConfidentialRepository>(cfg: &mut web::ServiceConfig) {
  cfg.service(
    web::scope("/v1")
      .configure(users::service::<R>)
      .configure(assets::service::<R>)
      .configure(accounts::service::<R>),
  );
}
