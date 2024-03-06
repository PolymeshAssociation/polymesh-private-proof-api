use actix_web::web::Data;

use async_trait::async_trait;
use polymesh-private-proof-shared::{error::Result, CreateSigner, SignerInfo};

use polymesh_api::client::Signer;

mod db;
pub use db::SqliteSigningManager;

mod vault;
pub use vault::VaultSigningManager;

pub type AppSigningManager = Data<dyn SigningManagerTrait>;
pub type TxSigner = Box<dyn Signer>;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SigningManagerTrait: Send + Sync + 'static {
  // Signers
  async fn get_signers(&self) -> Result<Vec<SignerInfo>>;
  async fn get_signer_info(&self, signer: &str) -> Result<Option<SignerInfo>>;
  async fn get_signer(&self, signer: &str) -> Result<Option<TxSigner>>;
  async fn create_signer(&self, signer: &CreateSigner) -> Result<SignerInfo>;
}
