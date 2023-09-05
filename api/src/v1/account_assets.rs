use actix_web::{
    web,
    HttpResponse,
    Responder, Result,
};

use confidential_assets_api_shared::{
    CreateAccountAsset, AccountMintAsset, AccountAssetWithTx,
    SenderProofRequest,
    ReceiverVerifyRequest,
};

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
            .route("/{asset_id}/send", web::post().to(request_sender_proof::<R>))
            .route("/{asset_id}/receiver_verify", web::post().to(receiver_verify_request::<R>))
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
    let init = account.init_balance(create_account_asset.asset_id);

    // Save initialize account balance.
    let account_asset = match repo.create_account_asset(&init).await {
        Ok(account_asset) => account_asset,
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!("Internal server error: {:?}", e));
        }
    };

    // Return account_asset.
    HttpResponse::Ok().json(account_asset)
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

    // Mint asset.
    let update = match account_asset.mint(account_mint_asset.amount) {
        Some(update) => update,
        None => {
            return HttpResponse::InternalServerError().body("Failed to generate asset mint proof");
        }
    };

    // Update account balance.
    let account_asset = match repo.update_account_asset(&update).await {
        Ok(Some(account_asset)) => account_asset,
        Ok(None) => {
            return HttpResponse::InternalServerError().body("Internal server error: Failed to updated account asset.");
        }
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!("Internal server error: {:?}", e));
        }
    };

    // Return account_asset.
    HttpResponse::Ok().json(account_asset)
}

async fn request_sender_proof<R: MercatRepository>(
    path: web::Path<(i64, i64)>,
    req: web::Json<SenderProofRequest>,
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

    // Generate sender proof.
    let (update, tx) = match account_asset.create_send_tx(&req) {
        Ok(tx) => tx,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to generate sender proof: {e:?}"));
        }
    };

    // Update account balance.
    let account_asset = match repo.update_account_asset(&update).await {
        Ok(Some(account_asset)) => account_asset,
        Ok(None) => {
            return HttpResponse::InternalServerError().body("Internal server error: Failed to updated account asset.");
        }
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!("Internal server error: {:?}", e));
        }
    };

    // Return account_asset with sender proof.
    let balance_with_tx = AccountAssetWithTx::new_send_tx(account_asset, tx);
    HttpResponse::Ok().json(balance_with_tx)
}

async fn receiver_verify_request<R: MercatRepository>(
    path: web::Path<(i64, i64)>,
    req: web::Json<ReceiverVerifyRequest>,
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

    // Verify the sender's proof.
    match account_asset.receiver_verify_tx(&req) {
        Ok(is_valid) => {
            return HttpResponse::Ok().json(is_valid);
        },
        Err(e) => {
            return HttpResponse::InternalServerError()
                .body(format!("Sender proof verification failed: {e:?}"));
        }
    }

    /*
    // TODO: Update receiver balance?
    // Update account balance.
    let account_asset = match repo.update_account_asset(&update).await {
        Ok(Some(account_asset)) => account_asset,
        Ok(None) => {
            return HttpResponse::InternalServerError().body("Internal server error: Failed to updated account asset.");
        }
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!("Internal server error: {:?}", e));
        }
    };

    // Return account_asset with sender proof.
    let balance_with_tx = AccountAssetWithTx::new_send_tx(account_asset, tx);
    HttpResponse::Ok().json(balance_with_tx)
    */
}
