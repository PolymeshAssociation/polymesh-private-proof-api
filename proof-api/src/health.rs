use actix_web::{get, web, HttpResponse, Responder, Result};

pub const API_VERSION: &str = "v0.0.1";

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg.service(health_check);
}

#[get("/health")]
async fn health_check() -> Result<impl Responder> {
  Ok(
    HttpResponse::Ok()
      .append_header(("health-check", API_VERSION))
      .finish(),
  )
}
