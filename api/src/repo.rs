use async_trait::async_trait;
use confidential_assets_api_shared::{
  CreateUser, CreateAsset, CreateAccount, UpdateAccountAsset,
  User, Asset, Account, AccountWithSecret, AccountAsset,
  AccountAssetWithSecret,
};

mod sqlite;

pub use sqlite::SqliteMercatRepository;

pub type MercatRepoError = String;
pub type MercatRepoResult<T> = Result<T, MercatRepoError>;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait MercatRepository: Send + Sync + 'static {
    // Users
    async fn get_users(&self) -> MercatRepoResult<Vec<User>>;
    async fn get_user(&self, user_id: i64) -> MercatRepoResult<User>;
    async fn create_user(&self, user: &CreateUser) -> MercatRepoResult<User>;

    // Assets
    async fn get_assets(&self) -> MercatRepoResult<Vec<Asset>>;
    async fn get_asset(&self, asset_id: i64) -> MercatRepoResult<Asset>;
    async fn create_asset(&self, asset: &CreateAsset) -> MercatRepoResult<Asset>;

    // Accounts
    async fn get_accounts(&self) -> MercatRepoResult<Vec<Account>>;
    async fn get_account(&self, account_id: i64) -> MercatRepoResult<Account>;
    async fn get_account_with_secret(&self, account_id: i64) -> MercatRepoResult<AccountWithSecret>;
    async fn create_account(&self, account: &CreateAccount) -> MercatRepoResult<Account>;

    // Account balances
    async fn get_account_assets(&self, account_id: i64) -> MercatRepoResult<Vec<AccountAsset>>;
    async fn get_account_asset(&self, account_id: i64, asset_id: i64) -> MercatRepoResult<AccountAsset>;
    async fn get_account_asset_with_secret(&self, account_id: i64, asset_id: i64) -> MercatRepoResult<AccountAssetWithSecret>;
    async fn create_account_asset(&self, account_asset: &UpdateAccountAsset) -> MercatRepoResult<AccountAsset>;
    async fn update_account_asset(&self, account_asset: &UpdateAccountAsset) -> MercatRepoResult<Option<AccountAsset>>;
}
