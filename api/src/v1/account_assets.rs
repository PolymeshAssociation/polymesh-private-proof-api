use actix_web::{
    web,
    HttpResponse,
    Responder, Result,
};

use mercat_api_shared::{CreateAccountAsset, AccountAssetWithInitTx, AccountMintAsset, AccountAssetWithMintTx};

use crate::repo::MercatRepository;

pub fn service<R: MercatRepository>(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/assets")
            // GET
            .route("", web::get().to(get_all::<R>))
            .route("/{asset_id}", web::get().to(get::<R>))
            // POST
            .route("", web::post().to(post::<R>))
            .route("/{asset_id}/mint", web::post().to(post_mint::<R>))
    );
}

async fn get_all<R: MercatRepository>(account_id: web::Path<i64>, repo: web::Data<R>) -> Result<impl Responder> {
    Ok(match repo.get_account_assets(*account_id).await {
        Ok(account_assets) => HttpResponse::Ok().json(account_assets),
        Err(e) => HttpResponse::NotFound().body(format!("Internal server error: {:?}", e)),
    })
}

async fn get<R: MercatRepository>(path: web::Path<(i64, i64)>, repo: web::Data<R>) -> HttpResponse {
    let (account_id, asset_id) = path.into_inner();
    match repo.get_account_asset(account_id, asset_id).await {
        Ok(account_asset) => HttpResponse::Ok().json(account_asset),
        Err(_) => HttpResponse::NotFound().body("Not found"),
    }
}

async fn post<R: MercatRepository>(
    account_id: web::Path<i64>,
    create_account_asset: web::Json<CreateAccountAsset>,
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
    let (create, init_tx) = match account.init_balance_tx(create_account_asset.asset_id) {
        Some(tx) => tx,
        None => {
            return HttpResponse::InternalServerError().body("Failed to generate account initialization proof");
        }
    };

    // Save initialize account balance.
    let account_asset = match repo.create_account_asset(&create).await {
        Ok(account_asset) => account_asset,
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!("Internal server error: {:?}", e));
        }
    };

    // Return account_asset with init tx.
    let balance_with_tx = AccountAssetWithInitTx::new(account_asset, init_tx);
    HttpResponse::Ok().json(balance_with_tx)
}

async fn post_mint<R: MercatRepository>(
    path: web::Path<(i64, i64)>,
    account_mint_asset: web::Json<AccountMintAsset>,
    repo: web::Data<R>,
) -> HttpResponse {
    let (account_id, asset_id) = path.into_inner();
    // Get the account asset with account secret key.
    let account_asset = match repo.get_account_asset_with_secret(account_id, asset_id).await {
        Ok(account_asset) => account_asset,
        Err(_) => {
            return HttpResponse::NotFound().body("Account Asset not found");
        }
    };

    // Generate Asset mint proof.
    let (update, mint_tx) = match account_asset.create_mint_tx(account_mint_asset.amount) {
        Some(tx) => tx,
        None => {
            return HttpResponse::InternalServerError().body("Failed to generate account initialization proof");
        }
    };

    // Update account balance.
    let account_asset = match repo.create_account_asset(&update).await {
        Ok(account_asset) => account_asset,
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!("Internal server error: {:?}", e));
        }
    };

    // Return account_asset with mint tx.
    let balance_with_tx = AccountAssetWithMintTx::new(account_asset, mint_tx);
    HttpResponse::Ok().json(balance_with_tx)
}
