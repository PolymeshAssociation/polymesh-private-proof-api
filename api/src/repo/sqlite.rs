use async_trait::async_trait;
use mercat_api_shared::{
  CreateUser, CreateAsset, CreateAccount, UpdateAccountAsset,
  User, Asset, Account, AccountWithSecret, AccountAsset,
  AccountAssetWithSecret,
};

use super::{MercatRepository, MercatRepoResult};

pub struct SqliteMercatRepository {
    pool: sqlx::SqlitePool,
}

impl SqliteMercatRepository {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MercatRepository for SqliteMercatRepository {
    async fn get_users(&self) -> MercatRepoResult<Vec<User>> {
        sqlx::query_as!(User, r#"SELECT * FROM users"#,)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())
    }

    async fn get_user(&self, user_id: i64) -> MercatRepoResult<User> {
        sqlx::query_as!(
            User,
            r#"SELECT * FROM users WHERE user_id = ?"#,
            user_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    async fn create_user(&self, user: &CreateUser) -> MercatRepoResult<User> {
        sqlx::query_as!(User,
            r#"
      INSERT INTO users (username)
      VALUES (?)
      RETURNING user_id, username, created_at, updated_at
      "#,
        user.username,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    async fn get_assets(&self) -> MercatRepoResult<Vec<Asset>> {
        sqlx::query_as!(Asset, r#"SELECT * FROM assets"#,)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())
    }

    async fn get_asset(&self, asset_id: i64) -> MercatRepoResult<Asset> {
        sqlx::query_as!(
            Asset,
            r#"SELECT * FROM assets WHERE asset_id = ?"#,
            asset_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    async fn create_asset(&self, asset: &CreateAsset) -> MercatRepoResult<Asset> {
        sqlx::query_as!(Asset,
            r#"
      INSERT INTO assets (ticker)
      VALUES (?)
      RETURNING asset_id, ticker, created_at, updated_at
      "#,
        asset.ticker,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    async fn get_accounts(&self) -> MercatRepoResult<Vec<Account>> {
        sqlx::query_as!(Account,
          r#"SELECT account_id, public_key, created_at, updated_at FROM accounts"#,)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())
    }

    async fn get_account(&self, account_id: i64) -> MercatRepoResult<Account> {
        sqlx::query_as!(
            Account,
            r#"SELECT account_id, public_key, created_at, updated_at FROM accounts WHERE account_id = ?"#,
            account_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    async fn get_account_with_secret(&self, account_id: i64) -> MercatRepoResult<AccountWithSecret> {
        sqlx::query_as!(
            AccountWithSecret,
            r#"SELECT account_id, public_key, secret_key FROM accounts WHERE account_id = ?"#,
            account_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    async fn create_account(&self, account: &CreateAccount) -> MercatRepoResult<Account> {
        sqlx::query_as!(Account,
            r#"
      INSERT INTO accounts (public_key, secret_key)
      VALUES (?, ?)
      RETURNING account_id, public_key, created_at, updated_at
      "#,
        account.public_key,
        account.secret_key,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    async fn get_account_assets(&self, account_id: i64) -> MercatRepoResult<Vec<AccountAsset>> {
        sqlx::query_as!(
            AccountAsset,
            r#"SELECT * FROM account_assets WHERE account_id = ?"#,
            account_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    async fn get_account_asset(&self, account_id: i64, asset_id: i64) -> MercatRepoResult<AccountAsset> {
        sqlx::query_as!(
            AccountAsset,
            r#"SELECT * FROM account_assets WHERE account_id = ? AND asset_id = ?"#,
            account_id, asset_id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    async fn get_account_asset_with_secret(&self, account_id: i64, asset_id: i64) -> MercatRepoResult<AccountAssetWithSecret> {
        sqlx::query_as(r#"
          SELECT aa.account_asset_id, aa.asset_id, aa.balance, aa.enc_balance,
            acc.account_id, acc.public_key, acc.secret_key
          FROM account_assets as aa
          JOIN accounts as acc using(account_id)
          WHERE aa.account_id = ? AND aa.asset_id = ?
        "#)
        .bind(account_id)
        .bind(asset_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    async fn create_account_asset(&self, account_asset: &UpdateAccountAsset) -> MercatRepoResult<AccountAsset> {
        let balance = account_asset.balance as i64;
        let enc_balance = account_asset.enc_balance();
        sqlx::query_as!(AccountAsset,
            r#"
      INSERT INTO account_assets (account_id, asset_id, balance, enc_balance)
      VALUES (?, ?, ?, ?)
      RETURNING account_asset_id, account_id, asset_id, balance, enc_balance, created_at, updated_at
      "#,
        account_asset.account_id,
        account_asset.asset_id,
        balance,
        enc_balance,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    async fn update_account_asset(&self, account_asset: &UpdateAccountAsset) -> MercatRepoResult<Option<AccountAsset>> {
        let balance = account_asset.balance as i64;
        let enc_balance = account_asset.enc_balance();
        sqlx::query_as!(AccountAsset,
            r#"
      UPDATE account_assets SET balance = ?, enc_balance = ?, updated_at = CURRENT_TIMESTAMP
      WHERE account_id = ? AND asset_id = ?
      RETURNING account_asset_id as "account_asset_id!", account_id, asset_id,
        balance, enc_balance, created_at, updated_at
      "#,
        balance,
        enc_balance,
        account_asset.account_id,
        account_asset.asset_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }
}
