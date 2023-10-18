use serde::{Deserialize, Serialize};

use utoipa::ToSchema;

use zeroize::{Zeroize, ZeroizeOnDrop};

#[cfg(feature = "backend")]
use sp_core::{
  crypto::Pair,
  sr25519
};

pub mod error;
pub use error::*;

mod tx;
pub use tx::*;

mod proofs;
pub use proofs::*;

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct Signer {
  #[schema(example = 1)]
  pub signer_id: i64,
  #[schema(example = "Alice")]
  pub signer_name: String,
  #[schema(example = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY")]
  pub public_key: String,

  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Zeroize, ZeroizeOnDrop)]
#[cfg(feature = "backend")]
pub struct SignerWithSecret {
  pub signer_id: i64,
  pub signer_name: String,
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
  pub signer_name: String,
  #[schema(example = "//Alice")]
  pub secret_uri: String,
}

#[cfg(feature = "backend")]
impl CreateSigner {
  pub fn as_signer_with_secret(&self) -> Result<SignerWithSecret> {
    let pair = sr25519::Pair::from_string(&self.secret_uri, None)?;
    Ok(SignerWithSecret {
      signer_name: self.signer_name.clone(),
      public_key: pair.public().to_string(),
      secret_key: pair.to_raw_vec(),
      ..Default::default()
    })
  }
}
