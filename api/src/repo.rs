use async_trait::async_trait;
use mercat_api_shared::{
  CreateUser, CreateAsset, CreateAccount, CreateAccountBalance,
  User, Asset, Account, AccountBalance,
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
    async fn get_user(&self, id: i64) -> MercatRepoResult<User>;
    async fn create_user(&self, user: &CreateUser) -> MercatRepoResult<User>;

    // Assets
    async fn get_assets(&self) -> MercatRepoResult<Vec<Asset>>;
    async fn get_asset(&self, id: i64) -> MercatRepoResult<Asset>;
    async fn create_asset(&self, asset: &CreateAsset) -> MercatRepoResult<Asset>;

    // Accounts
    async fn get_accounts(&self) -> MercatRepoResult<Vec<Account>>;
    async fn get_account(&self, id: i64) -> MercatRepoResult<Account>;
    async fn create_account(&self, account: &CreateAccount) -> MercatRepoResult<Account>;

    // Account balances
    async fn get_account_balances(&self, account_id: i64) -> MercatRepoResult<Vec<AccountBalance>>;
    async fn get_account_balance(&self, account_id: i64, asset_id: i64) -> MercatRepoResult<AccountBalance>;
    async fn create_account_balance(&self, account_balance: &CreateAccountBalance) -> MercatRepoResult<AccountBalance>;
}
