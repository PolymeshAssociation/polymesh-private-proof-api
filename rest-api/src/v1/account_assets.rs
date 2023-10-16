use actix_web::{get, post, web, HttpResponse, Responder, Result};

use confidential_proof_shared::{
  AccountAssetWithTx, AccountMintAsset, CreateAccountAsset, ReceiverVerifyRequest,
  SenderProofRequest,
};

use crate::repo::Repository;

pub fn service(cfg: &mut web::ServiceConfig) {
  cfg
    .service(get_all_account_assets)
    .service(get_account_asset)
    .service(create_account_asset)
    .service(asset_issuer_mint)
    .service(request_sender_proof)
    .service(receiver_verify_request);
}

/// Get all assets for an account.
#[utoipa::path(
  responses(
    (status = 200, description = "List all assets for an account", body = [AccountAsset])
  )
)]
#[get("/accounts/{account_id}/assets")]
pub async fn get_all_account_assets(
  account_id: web::Path<i64>,
  repo: web::Data<Repository>,
) -> Result<impl Responder> {
  Ok(match repo.get_account_assets(*account_id).await {
    Ok(account_assets) => HttpResponse::Ok().json(account_assets),
    Err(e) => HttpResponse::NotFound().body(format!("Internal server error: {:?}", e)),
  })
}

/// Get one asset for the account.
#[utoipa::path(
  responses(
    (status = 200, description = "Get an asset for an account", body = AccountAsset)
  )
)]
#[get("/accounts/{account_id}/assets/{asset_id}")]
pub async fn get_account_asset(
  path: web::Path<(i64, i64)>,
  repo: web::Data<Repository>,
) -> HttpResponse {
  let (account_id, asset_id) = path.into_inner();
  match repo.get_account_asset(account_id, asset_id).await {
    Ok(account_asset) => HttpResponse::Ok().json(account_asset),
    Err(_) => HttpResponse::NotFound().body("Not found"),
  }
}

/// Add an asset to the account and initialize it's balance.
#[utoipa::path(
  responses(
    (status = 200, description = "Add an asset to the account and initialize it's balance", body = AccountAsset)
  )
)]
#[post("/accounts/{account_id}/assets")]
pub async fn create_account_asset(
  account_id: web::Path<i64>,
  create_account_asset: web::Json<CreateAccountAsset>,
  repo: web::Data<Repository>,
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

/// Asset issuer updates their account balance when minting.
#[utoipa::path(
  responses(
    (status = 200, description = "Asset issuer updates their account balance when minting", body = AccountAsset)
  )
)]
#[post("/accounts/{account_id}/assets/{asset_id}/mint")]
pub async fn asset_issuer_mint(
  path: web::Path<(i64, i64)>,
  account_mint_asset: web::Json<AccountMintAsset>,
  repo: web::Data<Repository>,
) -> HttpResponse {
  let (account_id, asset_id) = path.into_inner();
  // Get the account asset with account secret key.
  let account_asset = match repo
    .get_account_asset_with_secret(account_id, asset_id)
    .await
  {
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
      return HttpResponse::InternalServerError()
        .body("Internal server error: Failed to updated account asset.");
    }
    Err(e) => {
      return HttpResponse::InternalServerError().body(format!("Internal server error: {:?}", e));
    }
  };

  // Return account_asset.
  HttpResponse::Ok().json(account_asset)
}

/// Generate a sender proof.
#[utoipa::path(
  responses(
    (status = 200, description = "Generate a sender proof", body = AccountAssetWithTx)
  )
)]
#[post("/accounts/{account_id}/assets/{asset_id}/send")]
pub async fn request_sender_proof(
  path: web::Path<(i64, i64)>,
  req: web::Json<SenderProofRequest>,
  repo: web::Data<Repository>,
) -> HttpResponse {
  let (account_id, asset_id) = path.into_inner();
  // Get the account asset with account secret key.
  let account_asset = match repo
    .get_account_asset_with_secret(account_id, asset_id)
    .await
  {
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
      return HttpResponse::InternalServerError()
        .body("Internal server error: Failed to updated account asset.");
    }
    Err(e) => {
      return HttpResponse::InternalServerError().body(format!("Internal server error: {:?}", e));
    }
  };

  // Return account_asset with sender proof.
  let balance_with_tx = AccountAssetWithTx::new_send_tx(account_asset, tx);
  HttpResponse::Ok().json(balance_with_tx)
}

/// Verify a sender proof as the receiver.
#[utoipa::path(
  responses(
    (status = 200, description = "Verify a sender proof as the receiver", body = bool)
  )
)]
#[post("/accounts/{account_id}/assets/{asset_id}/receiver_verify")]
pub async fn receiver_verify_request(
  path: web::Path<(i64, i64)>,
  req: web::Json<ReceiverVerifyRequest>,
  repo: web::Data<Repository>,
) -> HttpResponse {
  let (account_id, asset_id) = path.into_inner();
  // Get the account asset with account secret key.
  let account_asset = match repo
    .get_account_asset_with_secret(account_id, asset_id)
    .await
  {
    Ok(account_asset) => account_asset,
    Err(_) => {
      return HttpResponse::NotFound().body("Account Asset not found");
    }
  };

  // Verify the sender's proof.
  match account_asset.receiver_verify_tx(&req) {
    Ok(is_valid) => {
      return HttpResponse::Ok().json(is_valid);
    }
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
