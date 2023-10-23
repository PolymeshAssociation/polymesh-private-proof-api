use async_trait::async_trait;
use confidential_proof_shared::{error::Result, Signer, SignerWithSecret};

mod db;

pub use db::SqliteSigningManager;

pub type SigningManager = Box<dyn SigningManagerTrait>;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SigningManagerTrait: Send + Sync + 'static {
  // Signers
  async fn get_signers(&self) -> Result<Vec<Signer>>;
  async fn get_signer(&self, signer: &str) -> Result<Option<Signer>>;
  async fn get_signer_with_secret(&self, signer: &str) -> Result<Option<SignerWithSecret>>;
  async fn create_signer(&self, signer: &SignerWithSecret) -> Result<Signer>;
}
