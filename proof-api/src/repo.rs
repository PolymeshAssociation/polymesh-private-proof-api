use actix_web::web::Data;
use uuid::Uuid;

use async_trait::async_trait;
use polymesh_private_proof_shared::{
  error::Result, Account, AccountAsset, AccountAssetWithSecret, AccountWithSecret, AddAsset, Asset,
  CreateAccount, CreateUser, UpdateAccountAsset, User,
};

mod sqlite;

pub use sqlite::SqliteConfidentialRepository;

pub type Repository = Data<dyn ConfidentialRepository>;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ConfidentialRepository: Send + Sync + 'static {
  // Users
  async fn get_users(&self) -> Result<Vec<User>>;
  async fn get_user(&self, name: &str) -> Result<Option<User>>;
  async fn create_user(&self, user: &CreateUser) -> Result<User>;

  // Assets
  async fn get_assets(&self) -> Result<Vec<Asset>>;
  async fn get_asset(&self, asset_id: Uuid) -> Result<Option<Asset>>;
  async fn create_asset(&self, asset: &AddAsset) -> Result<Asset>;

  // Accounts
  async fn get_accounts(&self) -> Result<Vec<Account>>;
  async fn get_account(&self, pub_key: &str) -> Result<Option<Account>>;
  async fn get_account_with_secret(&self, pub_key: &str) -> Result<Option<AccountWithSecret>>;
  async fn create_account(&self, account: &CreateAccount) -> Result<Account>;

  // Account balances
  async fn get_account_assets(&self, pub_key: &str) -> Result<Vec<AccountAsset>>;
  async fn get_account_asset(&self, pub_key: &str, asset_id: Uuid) -> Result<Option<AccountAsset>>;
  async fn get_account_asset_with_secret(
    &self,
    pub_key: &str,
    asset_id: Uuid,
  ) -> Result<Option<AccountAssetWithSecret>>;
  async fn create_account_asset(&self, account_asset: &UpdateAccountAsset) -> Result<AccountAsset>;
  async fn update_account_asset(&self, account_asset: &UpdateAccountAsset) -> Result<AccountAsset>;
}
