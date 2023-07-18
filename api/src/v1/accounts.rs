use actix_web::{
    web,
    HttpResponse,
    Responder, Result,
};

use mercat_api_shared::CreateAccount;

use crate::repo::MercatRepository;
use super::account_balances;

fn account_service<R: MercatRepository>(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/{account_id}")
            // GET
            .route("", web::get().to(get::<R>))
            .configure(account_balances::service::<R>)
    );
}

pub fn service<R: MercatRepository>(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/accounts")
            // GET
            .route("", web::get().to(get_all::<R>))
            .configure(account_service::<R>)
            // POST
            .route("", web::post().to(post::<R>))
    );
}

async fn get_all<R: MercatRepository>(repo: web::Data<R>) -> Result<impl Responder> {
    Ok(match repo.get_accounts().await {
        Ok(accounts) => HttpResponse::Ok().json(accounts),
        Err(e) => HttpResponse::NotFound().body(format!("Internal server error: {:?}", e)),
    })
}

async fn get<R: MercatRepository>(account_id: web::Path<i64>, repo: web::Data<R>) -> HttpResponse {
    match repo.get_account(*account_id).await {
        Ok(account) => HttpResponse::Ok().json(account),
        Err(_) => HttpResponse::NotFound().body("Not found"),
    }
}

async fn post<R: MercatRepository>(
    repo: web::Data<R>,
) -> HttpResponse {
    let account = CreateAccount::new();
    match repo.create_account(&account).await {
        Ok(account) => HttpResponse::Ok().json(account),
        Err(e) => {
            HttpResponse::InternalServerError().body(format!("Internal server error: {:?}", e))
        }
    }
}
