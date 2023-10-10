use actix_web::{web, HttpResponse, Responder, Result};

use confidential_assets_api_shared::CreateUser;

use crate::repo::MercatRepository;

pub fn service<R: MercatRepository>(cfg: &mut web::ServiceConfig) {
  cfg.service(
    web::scope("/users")
      // GET
      .route("", web::get().to(get_all::<R>))
      .route("/{user_id}", web::get().to(get::<R>))
      // POST
      .route("", web::post().to(post::<R>)),
  );
}

async fn get_all<R: MercatRepository>(repo: web::Data<R>) -> Result<impl Responder> {
  Ok(match repo.get_users().await {
    Ok(users) => HttpResponse::Ok().json(users),
    Err(e) => HttpResponse::NotFound().body(format!("Internal server error: {:?}", e)),
  })
}

async fn get<R: MercatRepository>(user_id: web::Path<i64>, repo: web::Data<R>) -> HttpResponse {
  match repo.get_user(*user_id).await {
    Ok(user) => HttpResponse::Ok().json(user),
    Err(_) => HttpResponse::NotFound().body("Not found"),
  }
}

async fn post<R: MercatRepository>(
  user: web::Json<CreateUser>,
  repo: web::Data<R>,
) -> HttpResponse {
  match repo.create_user(&user).await {
    Ok(user) => HttpResponse::Ok().json(user),
    Err(e) => HttpResponse::InternalServerError().body(format!("Internal server error: {:?}", e)),
  }
}
