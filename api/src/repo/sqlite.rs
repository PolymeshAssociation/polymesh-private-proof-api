use async_trait::async_trait;
use mercat_api_shared::{
  CreateUser, CreateAsset, CreateAccount, CreateAccountBalance,
  User, Asset, Account, AccountBalance,
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
            r#"SELECT * FROM users WHERE id = ?"#,
            user_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    async fn create_user(&self, user: &CreateUser) -> MercatRepoResult<User> {
        sqlx::query_as!(User,
            r#"
      INSERT INTO users (name)
      VALUES (?)
      RETURNING id, name, created_at, updated_at
      "#,
        user.name,
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
            r#"SELECT * FROM assets WHERE id = ?"#,
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
      RETURNING id, ticker, created_at, updated_at
      "#,
        asset.ticker,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    async fn get_accounts(&self) -> MercatRepoResult<Vec<Account>> {
        sqlx::query_as!(Account, r#"SELECT * FROM accounts"#,)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())
    }

    async fn get_account(&self, account_id: i64) -> MercatRepoResult<Account> {
        sqlx::query_as!(
            Account,
            r#"SELECT * FROM accounts WHERE id = ?"#,
            account_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    async fn create_account(&self, account: &CreateAccount) -> MercatRepoResult<Account> {
        sqlx::query_as!(Account,
            r#"
      INSERT INTO accounts (public_key, enc_keys)
      VALUES (?, ?)
      RETURNING id, public_key, enc_keys, created_at, updated_at
      "#,
        account.public_key,
        account.enc_keys,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    async fn get_account_balances(&self, account_id: i64) -> MercatRepoResult<Vec<AccountBalance>> {
        sqlx::query_as!(
            AccountBalance,
            r#"SELECT * FROM account_balances WHERE id = ?"#,
            account_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    async fn get_account_balance(&self, account_id: i64, asset_id: i64) -> MercatRepoResult<AccountBalance> {
        sqlx::query_as!(
            AccountBalance,
            r#"SELECT * FROM account_balances WHERE account_id = ? AND asset_id = ?"#,
            account_id, asset_id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    async fn create_account_balance(&self, account_balance: &CreateAccountBalance) -> MercatRepoResult<AccountBalance> {
        let balance = account_balance.balance as i64;
        sqlx::query_as!(AccountBalance,
            r#"
      INSERT INTO account_balances (account_id, asset_id, balance, enc_balance)
      VALUES (?, ?, ?, ?)
      RETURNING id, account_id, asset_id, balance, enc_balance, created_at, updated_at
      "#,
        account_balance.account_id,
        account_balance.asset_id,
        balance,
        account_balance.enc_balance,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }
}
