use std::sync::Arc;

use uuid::Uuid;

use actix_web::web::Data;

use async_trait::async_trait;
use polymesh_private_proof_shared::{
  error::Result, Account, AccountAsset, AccountAssetWithSecret, AccountWithSecret, AddAsset, Asset,
  CreateAccount, CreateUser, PublicKey, UpdateAccountAsset, User,
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
      sqlx::query_as!(
        Asset,
        r#"
          SELECT asset_id as "asset_id: Uuid", created_at, updated_at
          FROM assets
"#,
      )
      .fetch_all(&self.pool)
      .await?,
    )
  }

  async fn get_asset(&self, asset_id: Uuid) -> Result<Option<Asset>> {
    Ok(
      sqlx::query_as!(
        Asset,
        r#"
        SELECT asset_id as "asset_id: Uuid", created_at, updated_at
        FROM assets WHERE asset_id = ?"#,
        asset_id
      )
      .fetch_optional(&self.pool)
      .await?,
    )
  }

  async fn create_asset(&self, asset: &AddAsset) -> Result<Asset> {
    Ok(
      sqlx::query_as!(
        Asset,
        r#"
      INSERT INTO assets (asset_id)
      VALUES (?)
      RETURNING asset_id as "asset_id: Uuid", created_at, updated_at
      "#,
        asset.asset_id,
      )
      .fetch_one(&self.pool)
      .await?,
    )
  }

  async fn get_accounts(&self) -> Result<Vec<Account>> {
    Ok(
      sqlx::query_as!(
        Account,
        r#"SELECT account_id, public_key as confidential_account, created_at, updated_at FROM accounts"#,
      )
      .fetch_all(&self.pool)
      .await?,
    )
  }

  async fn get_account(&self, pub_key: &str) -> Result<Option<Account>> {
    let pub_key = PublicKey::from_str(pub_key)?;
    let key = pub_key.0.as_slice();
    Ok(sqlx::query_as!(
      Account,
      r#"SELECT account_id, public_key as confidential_account, created_at, updated_at FROM accounts WHERE public_key = ?"#,
      key
    )
    .fetch_optional(&self.pool)
    .await?)
  }

  async fn get_account_with_secret(&self, pub_key: &str) -> Result<Option<AccountWithSecret>> {
    let pub_key = PublicKey::from_str(pub_key)?;
    let key = pub_key.0.as_slice();
    Ok(
      sqlx::query_as!(
        AccountWithSecret,
        r#"SELECT account_id, public_key as confidential_account, secret_key FROM accounts WHERE public_key = ?"#,
        key
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
      RETURNING account_id, public_key as confidential_account, created_at, updated_at
      "#,
        account.confidential_account,
        account.secret_key,
      )
      .fetch_one(&self.pool)
      .await?,
    )
  }

  async fn get_account_assets(&self, pub_key: &str) -> Result<Vec<AccountAsset>> {
    let pub_key = PublicKey::from_str(pub_key)?;
    let key = pub_key.0.as_slice();
    Ok(
      sqlx::query_as!(
        AccountAsset,
        r#"
          SELECT aa.asset_id as "asset_id: Uuid",
            aa.account_asset_id, aa.account_id,
            aa.balance, aa.enc_balance, aa.created_at, aa.updated_at
          FROM account_assets as aa
          JOIN accounts as acc using(account_id)
          WHERE acc.public_key = ?
        "#,
        key
      )
      .fetch_all(&self.pool)
      .await?,
    )
  }

  async fn get_account_asset(&self, pub_key: &str, asset_id: Uuid) -> Result<Option<AccountAsset>> {
    let pub_key = PublicKey::from_str(pub_key)?;
    let key = pub_key.0.as_slice();
    Ok(
      sqlx::query_as!(
        AccountAsset,
        r#"
          SELECT aa.asset_id as "asset_id: Uuid",
            aa.account_asset_id, aa.account_id,
            aa.balance, aa.enc_balance, aa.created_at, aa.updated_at
          FROM account_assets as aa
          JOIN accounts as acc using(account_id)
          WHERE acc.public_key = ? AND aa.asset_id = ?
        "#,
        key,
        asset_id,
      )
      .fetch_optional(&self.pool)
      .await?,
    )
  }

  async fn get_account_asset_with_secret(
    &self,
    pub_key: &str,
    asset_id: Uuid,
  ) -> Result<Option<AccountAssetWithSecret>> {
    let pub_key = PublicKey::from_str(pub_key)?;
    let key = pub_key.0.as_slice();
    Ok(
      sqlx::query_as(
        r#"
          SELECT aa.account_asset_id, aa.asset_id, aa.balance, aa.enc_balance,
            acc.account_id, acc.public_key, acc.secret_key
          FROM account_assets as aa
          JOIN accounts as acc using(account_id)
          WHERE acc.public_key = ? AND aa.asset_id = ?
        "#,
      )
      .bind(key)
      .bind(asset_id)
      .fetch_optional(&self.pool)
      .await?,
    )
  }

  async fn create_account_asset(&self, account_asset: &UpdateAccountAsset) -> Result<AccountAsset> {
    let mut conn = self.pool.acquire().await?;
    let balance = account_asset.balance as i64;
    let enc_balance = account_asset.enc_balance();
    let account = sqlx::query!(
      r#"
      INSERT INTO account_assets (account_id, asset_id, balance, enc_balance)
      VALUES (?, ?, ?, ?)
      ON CONFLICT(account_id, asset_id)
        DO UPDATE SET balance = excluded.balance, enc_balance = excluded.enc_balance, updated_at = CURRENT_TIMESTAMP
      RETURNING account_asset_id as id
      "#,
      account_asset.account_id,
      account_asset.asset_id,
      balance,
      enc_balance,
    )
    .fetch_one(conn.as_mut())
    .await?;
    Ok(
      sqlx::query_as!(
        AccountAsset,
        r#"
      SELECT asset_id as "asset_id: Uuid",
        account_asset_id, account_id,
        balance, enc_balance, created_at, updated_at
        FROM account_assets
        WHERE account_asset_id = ?
      "#,
        account.id,
      )
      .fetch_one(conn.as_mut())
      .await?,
    )
  }

  async fn update_account_asset(&self, account_asset: &UpdateAccountAsset) -> Result<AccountAsset> {
    let account_asset_id = if let Some(id) = account_asset.account_asset_id {
      id
    } else {
      return self.create_account_asset(account_asset).await;
    };
    let mut conn = self.pool.acquire().await?;
    let balance = account_asset.balance as i64;
    let enc_balance = account_asset.enc_balance();
    sqlx::query!(
      r#"
      UPDATE account_assets SET balance = ?, enc_balance = ?, updated_at = CURRENT_TIMESTAMP
        WHERE account_asset_id = ?
      RETURNING account_asset_id as id
      "#,
      balance,
      enc_balance,
      account_asset_id,
    )
    .fetch_optional(conn.as_mut())
    .await?;

    Ok(
      sqlx::query_as!(
        AccountAsset,
        r#"
      SELECT asset_id as "asset_id: Uuid",
        account_asset_id, account_id,
        balance, enc_balance, created_at, updated_at
        FROM account_assets
        WHERE account_asset_id = ?
      "#,
        account_asset_id,
      )
      .fetch_one(conn.as_mut())
      .await?,
    )
  }
}
