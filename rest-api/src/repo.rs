use async_trait::async_trait;
use confidential_proof_shared::{
  Account, AccountAsset, AccountAssetWithSecret, AccountWithSecret, Asset, CreateAccount,
  CreateAsset, CreateUser, UpdateAccountAsset, User,
  Signer, SignerWithSecret,
};

mod sqlite;

pub use sqlite::SqliteConfidentialRepository;

pub type ConfidentialRepoError = String;
pub type ConfidentialRepoResult<T> = Result<T, ConfidentialRepoError>;

pub type Repository = Box<dyn ConfidentialRepository>;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ConfidentialRepository: Send + Sync + 'static {
  // Users
  async fn get_users(&self) -> ConfidentialRepoResult<Vec<User>>;
  async fn get_user(&self, user_id: i64) -> ConfidentialRepoResult<User>;
  async fn create_user(&self, user: &CreateUser) -> ConfidentialRepoResult<User>;

  // Signers
  async fn get_signers(&self) -> ConfidentialRepoResult<Vec<Signer>>;
  async fn get_signer(&self, signer: &str) -> ConfidentialRepoResult<Signer>;
  async fn get_signer_with_secret(
    &self,
    signer: &str,
  ) -> ConfidentialRepoResult<SignerWithSecret>;
  async fn create_signer(&self, signer: &SignerWithSecret) -> ConfidentialRepoResult<Signer>;

  // Assets
  async fn get_assets(&self) -> ConfidentialRepoResult<Vec<Asset>>;
  async fn get_asset(&self, asset_id: i64) -> ConfidentialRepoResult<Asset>;
  async fn create_asset(&self, asset: &CreateAsset) -> ConfidentialRepoResult<Asset>;

  // Accounts
  async fn get_accounts(&self) -> ConfidentialRepoResult<Vec<Account>>;
  async fn get_account(&self, account_id: i64) -> ConfidentialRepoResult<Account>;
  async fn get_account_with_secret(
    &self,
    account_id: i64,
  ) -> ConfidentialRepoResult<AccountWithSecret>;
  async fn create_account(&self, account: &CreateAccount) -> ConfidentialRepoResult<Account>;

  // Account balances
  async fn get_account_assets(&self, account_id: i64) -> ConfidentialRepoResult<Vec<AccountAsset>>;
  async fn get_account_asset(
    &self,
    account_id: i64,
    asset_id: i64,
  ) -> ConfidentialRepoResult<AccountAsset>;
  async fn get_account_asset_with_secret(
    &self,
    account_id: i64,
    asset_id: i64,
  ) -> ConfidentialRepoResult<AccountAssetWithSecret>;
  async fn create_account_asset(
    &self,
    account_asset: &UpdateAccountAsset,
  ) -> ConfidentialRepoResult<AccountAsset>;
  async fn update_account_asset(
    &self,
    account_asset: &UpdateAccountAsset,
  ) -> ConfidentialRepoResult<Option<AccountAsset>>;
}
