use actix_web::web;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg.service(
    web::scope("/tx")
  );
}
