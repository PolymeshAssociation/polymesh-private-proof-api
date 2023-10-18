use async_trait::async_trait;
use confidential_proof_shared::{
  error::Result, Account, AccountAsset, AccountAssetWithSecret, AccountWithSecret, Asset,
  CreateAccount, CreateAsset, CreateUser, Signer, SignerWithSecret, UpdateAccountAsset, User,
};

mod sqlite;

pub use sqlite::SqliteConfidentialRepository;

pub type Repository = Box<dyn ConfidentialRepository>;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ConfidentialRepository: Send + Sync + 'static {
  // Users
  async fn get_users(&self) -> Result<Vec<User>>;
  async fn get_user(&self, user_id: i64) -> Result<Option<User>>;
  async fn create_user(&self, user: &CreateUser) -> Result<User>;

  // Signers
  async fn get_signers(&self) -> Result<Vec<Signer>>;
  async fn get_signer(&self, signer: &str) -> Result<Option<Signer>>;
  async fn get_signer_with_secret(&self, signer: &str) -> Result<Option<SignerWithSecret>>;
  async fn create_signer(&self, signer: &SignerWithSecret) -> Result<Signer>;

  // Assets
  async fn get_assets(&self) -> Result<Vec<Asset>>;
  async fn get_asset(&self, asset_id: i64) -> Result<Option<Asset>>;
  async fn create_asset(&self, asset: &CreateAsset) -> Result<Asset>;

  // Accounts
  async fn get_accounts(&self) -> Result<Vec<Account>>;
  async fn get_account(&self, account_id: i64) -> Result<Option<Account>>;
  async fn get_account_with_secret(&self, account_id: i64) -> Result<Option<AccountWithSecret>>;
  async fn create_account(&self, account: &CreateAccount) -> Result<Account>;

  // Account balances
  async fn get_account_assets(&self, account_id: i64) -> Result<Vec<AccountAsset>>;
  async fn get_account_asset(&self, account_id: i64, asset_id: i64)
    -> Result<Option<AccountAsset>>;
  async fn get_account_asset_with_secret(
    &self,
    account_id: i64,
    asset_id: i64,
  ) -> Result<Option<AccountAssetWithSecret>>;
  async fn create_account_asset(&self, account_asset: &UpdateAccountAsset) -> Result<AccountAsset>;
  async fn update_account_asset(
    &self,
    account_asset: &UpdateAccountAsset,
  ) -> Result<Option<AccountAsset>>;
}
