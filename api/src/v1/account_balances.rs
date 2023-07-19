use actix_web::{
    web,
    HttpResponse,
    Responder, Result,
};

use mercat_api_shared::{CreateAccountBalance, AccountBalanceWithInitTx};

use crate::repo::MercatRepository;

pub fn service<R: MercatRepository>(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/balances")
            // GET
            .route("", web::get().to(get_all::<R>))
            .route("/{asset_id}", web::get().to(get::<R>))
            // POST
            .route("", web::post().to(post::<R>))
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
    account_id: web::Path<i64>,
    mut create_balance: web::Json<CreateAccountBalance>,
    repo: web::Data<R>,
) -> HttpResponse {
    // Get the account's secret key.
    let account = match repo.get_account_with_secret(*account_id).await {
        Ok(account) => account,
        Err(_) => {
            return HttpResponse::NotFound().body("Account not found");
        }
    };

    // Generate Account initialization proof.
    let init_tx = match account.init_balance_tx() {
        Some(tx) => tx,
        None => {
            return HttpResponse::InternalServerError().body("Failed to generate account initialization proof");
        }
    };

    // Save initialize account balance.
    create_balance.account_id = account.account_id;
    create_balance.init_balance(&init_tx);
    let account_balance = match repo.create_account_balance(&create_balance).await {
        Ok(account_balance) => account_balance,
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!("Internal server error: {:?}", e));
        }
    };

    // Return account_balance with init tx.
    let balance_with_tx = AccountBalanceWithInitTx::new(account_balance, init_tx);
    HttpResponse::Ok().json(balance_with_tx)
}
