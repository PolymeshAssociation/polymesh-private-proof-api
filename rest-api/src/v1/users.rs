use actix_web::{get, post, web, HttpResponse, Responder, Result};

use confidential_proof_shared::{error::Error, CreateUser};

use crate::repo::Repository;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg
    .service(get_all_users)
    .service(get_user)
    .service(create_user);
}

/// Get all users.
#[utoipa::path(
  responses(
    (status = 200, body = [User])
  )
)]
#[get("/users")]
pub async fn get_all_users(repo: web::Data<Repository>) -> Result<impl Responder> {
  let users = repo.get_users().await?;
  Ok(HttpResponse::Ok().json(users))
}

/// Get one user.
#[utoipa::path(
  responses(
    (status = 200, body = User)
  )
)]
#[get("/users/{user_id}")]
pub async fn get_user(user_id: web::Path<i64>, repo: web::Data<Repository>) -> Result<impl Responder> {
  let user = repo.get_user(*user_id).await?
    .ok_or_else(|| Error::not_found("User"))?;
  Ok(HttpResponse::Ok().json(user))
}

/// Create a new user.
#[utoipa::path(
  responses(
    (status = 200, body = User)
  )
)]
#[post("/users")]
pub async fn create_user(user: web::Json<CreateUser>, repo: web::Data<Repository>) -> Result<impl Responder> {
  let user = repo.create_user(&user).await?;
  Ok(HttpResponse::Ok().json(user))
}
