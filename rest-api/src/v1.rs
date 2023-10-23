use actix_web::web;

pub mod signers;
pub mod tx;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg.service(
    web::scope("/v1")
      .configure(signers::service)
      .configure(tx::service),
  );
}
