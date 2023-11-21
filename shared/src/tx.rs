use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use utoipa::ToSchema;

use codec::{Decode, Encode};

#[cfg(feature = "backend")]
use polymesh_api::{
  client::{
    basic_types::{AccountId, IdentityId},
    block::EventRecords,
    ExtrinsicResult,
  },
  types::{
    pallet_confidential_asset::{
      AffirmParty, ConfidentialAccount, ConfidentialAuditors, ConfidentialTransactionRole,
      MediatorAccount, TransactionId, TransactionLeg, TransactionLegId,
    },
    polymesh_common_utilities::traits::checkpoint::ScheduleId,
    polymesh_primitives::{
      asset::CheckpointId,
      settlement::{InstructionId, VenueId},
      ticker::Ticker,
      Memo,
    },
    runtime::{events::*, RuntimeEvent},
  },
  TransactionResults,
};

#[cfg(feature = "backend")]
use confidential_assets::{Balance, ElgamalPublicKey};

use crate::error::Result;
use crate::proofs::{PublicKey, SenderProof};

pub fn scale_convert<T1: Encode, T2: Decode>(t1: &T1) -> T2 {
  let buf = t1.encode();
  T2::decode(&mut &buf[..]).expect("The two types don't have compatible SCALE encoding")
}

pub fn confidential_account_to_key(account: &ConfidentialAccount) -> ElgamalPublicKey {
  scale_convert(account)
}

pub fn mediator_account_to_key(account: &MediatorAccount) -> ElgamalPublicKey {
  scale_convert(account)
}

/// A Confidential asset transaction was affirmed.
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct TransactionAffirmed {
  /// Confidential transaction id.
  #[schema(value_type = u64)]
  pub transaction_id: TransactionId,
  /// Confidential transaction pending affirmations.
  #[schema(value_type = u32)]
  pub pending_affirms: u32,
  /// Confidential transaction leg id.
  #[schema(value_type = u64)]
  pub leg_id: TransactionLegId,
  /// Confidential transaction leg sender proof (if the sender affirmed).
  #[schema(value_type = String, format = Binary, example = "<Hex encoded sender proof>")]
  pub sender_proof: Option<SenderProof>,
}

/// Processed event from the transaction.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub enum ProcessedEvent {
  /// An identity was created.
  #[schema(value_type = Object, example = json!(Self::IdentityCreated(Default::default())))]
  IdentityCreated(IdentityId),
  /// A child identity was created.
  #[schema(value_type = [u8; 32], example = json!(Self::ChildIdentityCreated(Default::default())))]
  ChildIdentityCreated(IdentityId),
  /// A MultiSig was created.
  #[schema(value_type = [u8; 32], example = json!(Self::MultiSigCreated(Default::default())))]
  MultiSigCreated(AccountId),
  /// A Settlement Venue was created.
  #[schema(value_type = u64)]
  VenueCreated(VenueId),
  /// A Settlement instruction was created.
  #[schema(value_type = u64)]
  InstructionCreated(InstructionId),
  /// An asset checkpoint was created.
  #[schema(value_type = u64)]
  CheckpointCreated(CheckpointId),
  /// An asset checkpoint schedule was created.
  #[schema(value_type = u64)]
  ScheduleCreated(ScheduleId),
  /// A Confidential asset Venue was created.
  #[schema(value_type = u64)]
  ConfidentialVenueCreated(VenueId),
  /// A Confidential asset transaction was created.
  #[schema(value_type = u64)]
  ConfidentialTransactionCreated(TransactionId),
  /// A Confidential asset transaction was affirmed.
  ConfidentialTransactionAffirmed(TransactionAffirmed),
}

/// Processed events from the transaction.
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct ProcessedEvents(pub Vec<ProcessedEvent>);

impl ProcessedEvents {
  /// Get ids from *Created events.
  pub fn from_events(events: &EventRecords<RuntimeEvent>) -> Result<Self> {
    let mut processed = Vec::new();
    for rec in &events.0 {
      match &rec.event {
        RuntimeEvent::Settlement(SettlementEvent::VenueCreated(_, id, ..)) => {
          processed.push(ProcessedEvent::VenueCreated(*id));
        }
        RuntimeEvent::Settlement(SettlementEvent::InstructionCreated(_, _, id, ..)) => {
          processed.push(ProcessedEvent::InstructionCreated(*id));
        }
        RuntimeEvent::Checkpoint(CheckpointEvent::CheckpointCreated(_, _, id, ..)) => {
          processed.push(ProcessedEvent::CheckpointCreated(id.clone()));
        }
        RuntimeEvent::Checkpoint(CheckpointEvent::ScheduleCreated(_, _, id, ..)) => {
          processed.push(ProcessedEvent::ScheduleCreated(id.clone()));
        }
        RuntimeEvent::Identity(IdentityEvent::DidCreated(id, ..)) => {
          processed.push(ProcessedEvent::IdentityCreated(*id));
        }
        RuntimeEvent::Identity(IdentityEvent::ChildDidCreated(_, id, ..)) => {
          processed.push(ProcessedEvent::ChildIdentityCreated(*id));
        }
        RuntimeEvent::MultiSig(MultiSigEvent::MultiSigCreated(_, id, ..)) => {
          processed.push(ProcessedEvent::MultiSigCreated(*id));
        }
        RuntimeEvent::ConfidentialAsset(ConfidentialAssetEvent::VenueCreated(_, id)) => {
          processed.push(ProcessedEvent::ConfidentialVenueCreated(*id));
        }
        RuntimeEvent::ConfidentialAsset(ConfidentialAssetEvent::TransactionCreated(
          _,
          _,
          id,
          ..,
        )) => {
          processed.push(ProcessedEvent::ConfidentialTransactionCreated(*id));
        }
        RuntimeEvent::ConfidentialAsset(ConfidentialAssetEvent::TransactionAffirmed(
          _,
          tx_id,
          leg_id,
          AffirmParty::Sender(sender_proof),
          pending,
        )) => {
          processed.push(ProcessedEvent::ConfidentialTransactionAffirmed(
            TransactionAffirmed {
              transaction_id: *tx_id,
              pending_affirms: *pending,
              leg_id: *leg_id,
              sender_proof: Some(SenderProof(sender_proof.encode())),
            },
          ));
        }
        RuntimeEvent::ConfidentialAsset(ConfidentialAssetEvent::TransactionAffirmed(
          _,
          tx_id,
          leg_id,
          _,
          pending,
        )) => {
          processed.push(ProcessedEvent::ConfidentialTransactionAffirmed(
            TransactionAffirmed {
              transaction_id: *tx_id,
              pending_affirms: *pending,
              leg_id: *leg_id,
              sender_proof: None,
            },
          ));
        }
        _ => (),
      }
    }
    Ok(Self(processed))
  }
}

/// Transaction results
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct TransactionResult {
  /// Was the transaction sucessful.
  #[schema(example = true)]
  pub success: bool,
  /// Block hash.
  #[schema(example = "0xea549dcdadacb5678e37a336e44c581ade562b696159bf8fd846fee7e7fe1dc3")]
  pub block_hash: String,
  /// If `success` is false, then provide an error message.
  #[schema(example = json!(null))]
  pub err_msg: Option<String>,
  /// Processed Events.
  #[schema(example = json!([]))]
  pub processed_events: ProcessedEvents,
}

#[cfg(feature = "backend")]
impl TransactionResult {
  pub async fn wait_for_results(mut tx_res: TransactionResults, finalize: bool) -> Result<Self> {
    let mut res = Self::default();

    // Wait for transaction to execute.
    let block_hash = if finalize {
      tx_res.wait_finalized().await?
    } else {
      tx_res.wait_in_block().await?
    }
    .unwrap_or_default();
    res.block_hash = format!("{block_hash:#x}");

    // Process events.
    if let Some(events) = tx_res.events().await? {
      res.processed_events = ProcessedEvents::from_events(events)?;
    }

    match tx_res.extrinsic_result().await? {
      Some(ExtrinsicResult::Success(_info)) => {
        res.success = true;
      }
      Some(ExtrinsicResult::Failed(_info, err)) => {
        res.err_msg = Some(format!("{err:?}"));
      }
      None => {
        res.err_msg = Some(format!("{:?}", tx_res.status()));
      }
    }
    Ok(res)
  }
}

/// The auditor's role.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, ToSchema)]
pub enum AuditorRole {
  #[default]
  Auditor,
  Mediator,
}

#[cfg(feature = "backend")]
impl AuditorRole {
  pub fn from(role: ConfidentialTransactionRole) -> Self {
    match role {
      ConfidentialTransactionRole::Auditor => Self::Auditor,
      ConfidentialTransactionRole::Mediator => Self::Mediator,
    }
  }

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

pub fn bytes_to_memo(val: &[u8]) -> Memo {
  let mut memo = [0u8; 32];
  for (idx, b) in val.iter().take(32).enumerate() {
    memo[idx] = *b;
  }
  Memo(memo)
}

pub fn str_to_memo(val: &str) -> Result<Memo> {
  if val.starts_with("0x") {
    let b = hex::decode(&val.as_bytes()[2..])?;
    Ok(bytes_to_memo(b.as_slice()))
  } else {
    Ok(bytes_to_memo(val.as_bytes()))
  }
}

/// Confidential asset details (name, ticker, auditors).
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct ConfidentialAssetDetails {
  /// Asset name.
  #[schema(example = "Asset name")]
  pub name: String,
  /// Asset total supply.
  #[schema(example = "10000")]
  pub total_supply: u64,
  /// Asset owner.
  #[schema(example = json!(IdentityId::default()))]
  pub owner: IdentityId,
  // TODO: AssetType.
  /// List of mediators.
  #[schema(example = json!(["0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114"]))]
  #[serde(default)]
  pub mediators: Vec<PublicKey>,
  /// List of auditors.
  #[schema(example = json!(["0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114"]))]
  #[serde(default)]
  pub auditors: Vec<PublicKey>,
}

/// Create confidential asset on-chain.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct CreateConfidentialAsset {
  /// Signer of the transaction.
  #[schema(example = "Alice")]
  pub signer: String,
  /// Wait for block finalization.
  #[schema(example = false)]
  #[serde(default)]
  pub finalize: bool,
  /// Asset name.
  #[schema(example = "Asset name")]
  pub name: String,
  /// Asset ticker.
  #[schema(example = "TICKER")]
  pub ticker: String,
  /// List of mediators.
  #[schema(example = json!(["0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114"]))]
  #[serde(default)]
  pub mediators: Vec<PublicKey>,
  /// List of auditors.
  #[schema(example = json!(["0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114"]))]
  #[serde(default)]
  pub auditors: Vec<PublicKey>,
}

#[cfg(feature = "backend")]
impl CreateConfidentialAsset {
  pub fn ticker(&self) -> Result<Ticker> {
    str_to_ticker(&self.ticker)
  }

  pub fn auditors(&self) -> Result<ConfidentialAuditors> {
    let mut auditors = BTreeMap::new();
    for key in &self.mediators {
      auditors.insert(
        key.as_mediator_account()?,
        ConfidentialTransactionRole::Mediator,
      );
    }
    for key in &self.auditors {
      auditors.insert(
        key.as_mediator_account()?,
        ConfidentialTransactionRole::Auditor,
      );
    }
    Ok(ConfidentialAuditors { auditors })
  }
}

/// Transaction signer.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct TransactionArgs {
  /// Signer of the transaction.
  #[schema(example = "Alice")]
  pub signer: String,
  /// Wait for block finalization.
  #[schema(example = false)]
  #[serde(default)]
  pub finalize: bool,
}

/// Confidential asset settlement leg.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct ConfidentialSettlementLeg {
  /// Ticker.
  #[schema(example = "TICKER")]
  pub ticker: String,
  /// Sender's public key.
  #[schema(value_type = String, format = Binary, example = "0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114")]
  sender: PublicKey,
  /// Receiver's public key.
  #[schema(value_type = String, format = Binary, example = "0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114")]
  receiver: PublicKey,
  /// List of mediators.
  #[schema(example = json!({"0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114": "Mediator"}))]
  #[serde(default)]
  pub mediators: BTreeSet<PublicKey>,
  /// List of auditors.
  #[schema(example = json!({"0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114": "Mediator"}))]
  #[serde(default)]
  pub auditors: BTreeSet<PublicKey>,
}

#[cfg(feature = "backend")]
impl ConfidentialSettlementLeg {
  pub fn ticker(&self) -> Result<Ticker> {
    str_to_ticker(&self.ticker)
  }

  pub fn sender(&self) -> Result<ConfidentialAccount> {
    Ok(self.sender.as_confidential_account()?)
  }

  pub fn receiver(&self) -> Result<ConfidentialAccount> {
    Ok(self.receiver.as_confidential_account()?)
  }

  pub fn auditors(&self) -> Result<ConfidentialAuditors> {
    let mut auditors = BTreeMap::new();
    for key in &self.mediators {
      auditors.insert(
        key.as_mediator_account()?,
        ConfidentialTransactionRole::Mediator,
      );
    }
    for key in &self.auditors {
      auditors.insert(
        key.as_mediator_account()?,
        ConfidentialTransactionRole::Auditor,
      );
    }
    Ok(ConfidentialAuditors { auditors })
  }
}

/// Create confidential asset settlement.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct CreateConfidentialSettlement {
  /// Signer of the transaction.
  #[schema(example = "Alice")]
  pub signer: String,
  /// Wait for block finalization.
  #[schema(example = false)]
  #[serde(default)]
  pub finalize: bool,
  /// Settlement legs.
  pub legs: Vec<ConfidentialSettlementLeg>,
  /// Settlement memo.
  #[schema(example = "")]
  #[serde(default)]
  pub memo: String,
}

impl CreateConfidentialSettlement {
  pub fn legs(&self) -> Result<Vec<TransactionLeg>> {
    let mut legs = Vec::new();
    for leg in &self.legs {
      legs.push(TransactionLeg {
        ticker: leg.ticker()?,
        sender: leg.sender()?,
        receiver: leg.receiver()?,
        auditors: leg.auditors()?,
      });
    }
    Ok(legs)
  }

  pub fn memo(&self) -> Result<Option<Memo>> {
    Ok(if self.memo.len() > 0 {
      Some(str_to_memo(&self.memo)?)
    } else {
      None
    })
  }
}

/// Affirm Confidential asset transaction leg as the sender/receiver.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct AffirmTransactionLegRequest {
  /// Signer of the transaction.
  #[schema(example = "Alice")]
  pub signer: String,
  /// Wait for block finalization.
  #[schema(example = false)]
  #[serde(default)]
  pub finalize: bool,
  /// Confidential transaction id.
  #[schema(value_type = u64)]
  pub transaction_id: TransactionId,
  /// Confidential transaction leg id.
  #[schema(value_type = u32)]
  pub leg_id: TransactionLegId,
  /// Transaction Amount.
  #[schema(example = 1000, value_type = u64)]
  pub amount: Balance,
}

/// Execute confidential asset settlement.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct ExecuteConfidentialSettlement {
  /// Signer of the transaction.
  #[schema(example = "Alice")]
  pub signer: String,
  /// Wait for block finalization.
  #[schema(example = false)]
  #[serde(default)]
  pub finalize: bool,
  /// Settlement leg count.
  #[schema(example = 10)]
  pub leg_count: u32,
}

/// Confidential asset mint request.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct MintRequest {
  /// Signer of the transaction.
  #[schema(example = "Alice")]
  pub signer: String,
  /// Wait for block finalization.
  #[schema(example = false)]
  #[serde(default)]
  pub finalize: bool,
  /// Amount to mint.
  #[schema(example = 1000, value_type = u64)]
  pub amount: Balance,
}

/// Allow venues.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct AllowVenues {
  /// Signer of the transaction.
  #[schema(example = "Alice")]
  pub signer: String,
  /// Wait for block finalization.
  #[schema(example = false)]
  #[serde(default)]
  pub finalize: bool,
  /// Venues to allow.
  #[schema(example = json!([1]))]
  pub venues: Vec<u64>,
}

#[cfg(feature = "backend")]
impl AllowVenues {
  pub fn venues(&self) -> Vec<VenueId> {
    self.venues.iter().map(|id| VenueId(*id)).collect()
  }
}
