use actix_web::{get, post, web, HttpResponse, Responder, Result};

use confidential_assets_api_shared::CreateUser;

use crate::repo::Repository;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg.service(
    web::scope("/users")
      .service(get_all_users)
      .service(get_user)
      .service(create_user),
  );
}

/// Get all users.
#[get("")]
async fn get_all_users(repo: web::Data<Repository>) -> Result<impl Responder> {
  Ok(match repo.get_users().await {
    Ok(users) => HttpResponse::Ok().json(users),
    Err(e) => HttpResponse::NotFound().body(format!("Internal server error: {:?}", e)),
  })
}

/// Get one user.
#[get("/{user_id}")]
async fn get_user(user_id: web::Path<i64>, repo: web::Data<Repository>) -> HttpResponse {
  match repo.get_user(*user_id).await {
    Ok(user) => HttpResponse::Ok().json(user),
    Err(_) => HttpResponse::NotFound().body("Not found"),
  }
}

/// Create a new user.
#[post("")]
async fn create_user(user: web::Json<CreateUser>, repo: web::Data<Repository>) -> HttpResponse {
  match repo.create_user(&user).await {
    Ok(user) => HttpResponse::Ok().json(user),
    Err(e) => HttpResponse::InternalServerError().body(format!("Internal server error: {:?}", e)),
  }
}
