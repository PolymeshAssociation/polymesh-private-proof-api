use std::sync::Arc;

use actix_web::web::Data;

use async_trait::async_trait;
use confidential_proof_shared::{
  error::Result, Account, AccountAsset, AccountAssetWithSecret, AccountWithSecret, Asset,
  CreateAccount, CreateAsset, CreateUser, UpdateAccountAsset, User,
  PublicKey,
};

use super::{ConfidentialRepository, Repository};

pub struct SqliteConfidentialRepository {
  pool: sqlx::SqlitePool,
}

impl SqliteConfidentialRepository {
  pub fn new(pool: &sqlx::SqlitePool) -> Arc<dyn ConfidentialRepository> {
    Arc::new(Self { pool: pool.clone() })
  }

  pub fn new_app_data(pool: &sqlx::SqlitePool) -> Repository {
    Data::from(Self::new(pool))
  }
}

#[async_trait]
impl ConfidentialRepository for SqliteConfidentialRepository {
  async fn get_users(&self) -> Result<Vec<User>> {
    Ok(
      sqlx::query_as!(User, r#"SELECT * FROM users"#,)
        .fetch_all(&self.pool)
        .await?,
    )
  }

  async fn get_user(&self, name: &str) -> Result<Option<User>> {
    Ok(
      sqlx::query_as!(User, r#"SELECT * FROM users WHERE username = ?"#, name)
        .fetch_optional(&self.pool)
        .await?,
    )
  }

  async fn create_user(&self, user: &CreateUser) -> Result<User> {
    Ok(
      sqlx::query_as!(
        User,
        r#"
      INSERT INTO users (username)
      VALUES (?)
      RETURNING user_id, username, created_at, updated_at
      "#,
        user.username,
      )
      .fetch_one(&self.pool)
      .await?,
    )
  }

  async fn get_assets(&self) -> Result<Vec<Asset>> {
    Ok(
      sqlx::query_as!(Asset, r#"SELECT * FROM assets"#,)
        .fetch_all(&self.pool)
        .await?,
    )
  }

  async fn get_asset(&self, ticker: &str) -> Result<Option<Asset>> {
    Ok(
      sqlx::query_as!(
        Asset,
        r#"SELECT * FROM assets WHERE ticker = ?"#,
        ticker
      )
      .fetch_optional(&self.pool)
      .await?,
    )
  }

  async fn create_asset(&self, asset: &CreateAsset) -> Result<Asset> {
    Ok(
      sqlx::query_as!(
        Asset,
        r#"
      INSERT INTO assets (ticker)
      VALUES (?)
      RETURNING asset_id, ticker, created_at, updated_at
      "#,
        asset.ticker,
      )
      .fetch_one(&self.pool)
      .await?,
    )
  }

  async fn get_accounts(&self) -> Result<Vec<Account>> {
    Ok(
      sqlx::query_as!(
        Account,
        r#"SELECT account_id, public_key, created_at, updated_at FROM accounts"#,
      )
      .fetch_all(&self.pool)
      .await?,
    )
  }

  async fn get_account(&self, pub_key: &PublicKey) -> Result<Option<Account>> {
    Ok(sqlx::query_as!(
      Account,
      r#"SELECT account_id, public_key, created_at, updated_at FROM accounts WHERE public_key = ?"#,
      pub_key.0
    )
    .fetch_optional(&self.pool)
    .await?)
  }

  async fn get_account_with_secret(&self, pub_key: &PublicKey) -> Result<Option<AccountWithSecret>> {
    Ok(
      sqlx::query_as!(
        AccountWithSecret,
        r#"SELECT account_id, public_key, secret_key FROM accounts WHERE public_key = ?"#,
        pub_key.0
      )
      .fetch_optional(&self.pool)
      .await?,
    )
  }

  async fn create_account(&self, account: &CreateAccount) -> Result<Account> {
    Ok(
      sqlx::query_as!(
        Account,
        r#"
      INSERT INTO accounts (public_key, secret_key)
      VALUES (?, ?)
      RETURNING account_id, public_key, created_at, updated_at
      "#,
        account.public_key,
        account.secret_key,
      )
      .fetch_one(&self.pool)
      .await?,
    )
  }

  async fn get_account_assets(&self, pub_key: &PublicKey) -> Result<Vec<AccountAsset>> {
    Ok(
      sqlx::query_as!(
        AccountAsset,
        r#"
          SELECT aa.*, assets.ticker
          FROM account_assets as aa
          JOIN accounts as acc using(account_id)
          JOIN assets using(asset_id)
          WHERE acc.public_key = ?
        "#,
        pub_key.0
      )
      .fetch_all(&self.pool)
      .await?,
    )
  }

  async fn get_account_asset(
    &self,
    pub_key: &PublicKey,
    ticker: &str,
  ) -> Result<Option<AccountAsset>> {
    Ok(
      sqlx::query_as!(
        AccountAsset,
        r#"
          SELECT aa.*, assets.ticker
          FROM account_assets as aa
          JOIN accounts as acc using(account_id)
          JOIN assets using(asset_id)
          WHERE acc.public_key = ? AND assets.ticker = ?
        "#,
        pub_key.0,
        ticker,
      )
      .fetch_optional(&self.pool)
      .await?,
    )
  }

  async fn get_account_asset_with_secret(
    &self,
    pub_key: &PublicKey,
    ticker: &str,
  ) -> Result<Option<AccountAssetWithSecret>> {
    Ok(
      sqlx::query_as(
        r#"
          SELECT aa.account_asset_id, aa.asset_id, aa.balance, aa.enc_balance,
            acc.account_id, acc.public_key, acc.secret_key
          FROM account_assets as aa
          JOIN accounts as acc using(account_id)
          JOIN assets using(asset_id)
          WHERE acc.public_key = ? AND assets.ticker = ?
        "#,
      )
      .bind(&pub_key.0)
      .bind(ticker)
      .fetch_optional(&self.pool)
      .await?,
    )
  }

  async fn create_account_asset(&self, account_asset: &UpdateAccountAsset) -> Result<AccountAsset> {
    let balance = account_asset.balance as i64;
    let enc_balance = account_asset.enc_balance();
    let account = sqlx::query!(r#"
      INSERT INTO account_assets (account_id, asset_id, balance, enc_balance)
      VALUES (?, ?, ?, ?)
      RETURNING account_asset_id as id
      "#,
        account_asset.account_id,
        account_asset.asset_id,
        balance,
        enc_balance,
      )
      .fetch_one(&self.pool)
      .await?;
    Ok(
      sqlx::query_as!(
        AccountAsset,
        r#"
      SELECT aa.*, assets.ticker
        FROM account_assets as aa
          JOIN assets using(asset_id)
        WHERE account_asset_id = ?
      "#,
        account.id,
      )
      .fetch_one(&self.pool)
      .await?,
    )
  }

  async fn update_account_asset(
    &self,
    account_asset: &UpdateAccountAsset,
  ) -> Result<Option<AccountAsset>> {
    let balance = account_asset.balance as i64;
    let enc_balance = account_asset.enc_balance();
    let account = sqlx::query!(
        r#"
      UPDATE account_assets SET balance = ?, enc_balance = ?, updated_at = CURRENT_TIMESTAMP
        WHERE account_id = ? AND asset_id = ?
      RETURNING account_asset_id as id
      "#,
        balance,
        enc_balance,
        account_asset.account_id,
        account_asset.asset_id,
      )
      .fetch_optional(&self.pool)
      .await?;

    if let Some(account) = account {
      Ok(
        Some(sqlx::query_as!(
          AccountAsset,
          r#"
        SELECT aa.*, assets.ticker
          FROM account_assets as aa
            JOIN assets using(asset_id)
          WHERE account_asset_id = ?
        "#,
          account.id,
        )
        .fetch_one(&self.pool)
        .await?,
      ))
    } else {
      Ok(None)
    }
  }
}
