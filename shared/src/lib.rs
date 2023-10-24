use serde::{Deserialize, Serialize};

use utoipa::ToSchema;

use zeroize::{Zeroize, ZeroizeOnDrop};

#[cfg(feature = "backend")]
use sp_core::{crypto::Pair, sr25519};

pub mod error;
pub use error::*;

mod tx;
pub use tx::*;

mod proofs;
pub use proofs::*;

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct SignerInfo {
  #[schema(example = "Alice")]
  pub name: String,
  #[schema(example = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY")]
  pub public_key: String,

  pub created_at: chrono::NaiveDateTime,
}

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Zeroize, ZeroizeOnDrop)]
#[cfg(feature = "backend")]
pub struct SignerWithSecret {
  pub name: String,
  pub public_key: String,
  pub secret_key: Vec<u8>,
}

#[cfg(feature = "backend")]
impl SignerWithSecret {
  pub fn keypair(&self) -> Result<sr25519::Pair> {
    Ok(sr25519::Pair::from_seed_slice(self.secret_key.as_slice())?)
  }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema, Zeroize, ZeroizeOnDrop)]
pub struct CreateSigner {
  #[schema(example = "Alice")]
  pub name: String,
  /// Only used for "DB" signing manager.  The "VAULT" signing manager doesn't support
  /// importing keys from a secret.
  #[schema(example = "//Alice")]
  pub secret_uri: Option<String>,
}

#[cfg(feature = "backend")]
impl CreateSigner {
  pub fn as_signer_with_secret(&self) -> Result<SignerWithSecret> {
    let pair = match &self.secret_uri {
      Some(secret_uri) => sr25519::Pair::from_string(secret_uri, None)?,
      None => sr25519::Pair::generate().0,
    };
    Ok(SignerWithSecret {
      name: self.name.clone(),
      public_key: pair.public().to_string(),
      secret_key: pair.to_raw_vec(),
      ..Default::default()
    })
  }
}
