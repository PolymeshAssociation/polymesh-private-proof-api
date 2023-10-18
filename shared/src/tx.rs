use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use utoipa::ToSchema;

#[cfg(feature = "backend")]
use polymesh_api::types::{
  pallet_confidential_asset::{ConfidentialTransactionRole, MediatorAccount},
  polymesh_primitives::ticker::Ticker,
};

use crate::error::Result;
use crate::proofs::PublicKey;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, ToSchema)]
pub enum AuditorRole {
  #[default]
  Auditor,
  Mediator,
}

#[cfg(feature = "backend")]
impl AuditorRole {
  pub fn into_role(&self) -> ConfidentialTransactionRole {
    match self {
      Self::Auditor => ConfidentialTransactionRole::Auditor,
      Self::Mediator => ConfidentialTransactionRole::Mediator,
    }
  }
}

pub fn bytes_to_ticker(val: &[u8]) -> Ticker {
  let mut ticker = [0u8; 12];
  for (idx, b) in val.iter().take(12).enumerate() {
    ticker[idx] = *b;
  }
  Ticker(ticker)
}

pub fn str_to_ticker(val: &str) -> Result<Ticker> {
  if val.starts_with("0x") {
    let b = hex::decode(&val.as_bytes()[2..])?;
    Ok(bytes_to_ticker(b.as_slice()))
  } else {
    Ok(bytes_to_ticker(val.as_bytes()))
  }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct CreateConfidentialAsset {
  #[schema(example = "Alice")]
  pub signer: String,
  #[schema(example = "Asset name")]
  pub name: String,
  #[schema(example = "TICKER")]
  pub ticker: String,
  /// List of auditors/mediators.
  #[schema(example = json!({"0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114": "Mediator"}))]
  #[serde(default)]
  auditors: BTreeMap<PublicKey, AuditorRole>,
}

#[cfg(feature = "backend")]
impl CreateConfidentialAsset {
  pub fn ticker(&self) -> Result<Ticker> {
    str_to_ticker(&self.ticker)
  }

  pub fn auditors(&self) -> Result<BTreeMap<MediatorAccount, ConfidentialTransactionRole>> {
    let mut auditors = BTreeMap::new();
    for (key, role) in &self.auditors {
      auditors.insert(key.as_mediator_account()?, role.into_role());
    }
    Ok(auditors)
  }
}
