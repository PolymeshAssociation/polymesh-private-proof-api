use actix_web::{
    web,
    HttpResponse,
    Responder, Result,
};

use mercat_api_shared::CreateAccountBalance;

use crate::repo::MercatRepository;

pub fn service<R: MercatRepository>(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/balances")
            // GET
            .route("", web::get().to(get_all::<R>))
            .route("/{asset_id}", web::get().to(get::<R>))
            // POST
            .route("/{asset_id}", web::post().to(post::<R>))
    );
}

async fn get_all<R: MercatRepository>(account_id: web::Path<i64>, repo: web::Data<R>) -> Result<impl Responder> {
    Ok(match repo.get_account_balances(*account_id).await {
        Ok(account_balances) => HttpResponse::Ok().json(account_balances),
        Err(e) => HttpResponse::NotFound().body(format!("Internal server error: {:?}", e)),
    })
}

async fn get<R: MercatRepository>(path: web::Path<(i64, i64)>, repo: web::Data<R>) -> HttpResponse {
    let (account_id, asset_id) = path.into_inner();
    match repo.get_account_balance(account_id, asset_id).await {
        Ok(account_balance) => HttpResponse::Ok().json(account_balance),
        Err(_) => HttpResponse::NotFound().body("Not found"),
    }
}

async fn post<R: MercatRepository>(
    path: web::Path<(i64, i64)>,
    repo: web::Data<R>,
) -> HttpResponse {
    let (account_id, asset_id) = path.into_inner();
    let account_balance = CreateAccountBalance {
      account_id,
      asset_id,
      ..Default::default()
    };
    match repo.create_account_balance(&account_balance).await {
        Ok(account_balance) => HttpResponse::Ok().json(account_balance),
        Err(e) => {
            HttpResponse::InternalServerError().body(format!("Internal server error: {:?}", e))
        }
    }
}
