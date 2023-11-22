use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use utoipa::ToSchema;

use codec::{Decode, Encode};

#[cfg(feature = "backend")]
use polymesh_api::{
  client::{
    basic_types::{AccountId, IdentityId},
    block::{Header, EventRecord, Phase, ExtrinsicV4},
    ExtrinsicResult,
    EnumInfo,
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
  Api,
  ChainApi,
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

pub fn join_auditors(mediators: &[PublicKey], auditors: &[PublicKey]) -> Result<ConfidentialAuditors> {
  let mut roles = BTreeMap::new();
  for key in mediators {
    roles.insert(
      key.as_mediator_account()?,
      ConfidentialTransactionRole::Mediator,
    );
  }
  for key in auditors {
    roles.insert(
      key.as_mediator_account()?,
      ConfidentialTransactionRole::Auditor,
    );
  }
  Ok(ConfidentialAuditors { auditors: roles })
}

pub fn split_auditors(auditors: &ConfidentialAuditors) -> (Vec<PublicKey>, Vec<PublicKey>) {
  let mediators = auditors.auditors.iter().filter_map(|(key, role)| match role {
    ConfidentialTransactionRole::Mediator => Some(PublicKey(key.encode())),
    _ => None,
  }).collect();
  let auditors = auditors.auditors.iter().filter_map(|(key, role)| match role {
    ConfidentialTransactionRole::Auditor => Some(PublicKey(key.encode())),
    _ => None,
  }).collect();
  (mediators, auditors)
}

/// Settlement record.
#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SettlementRecord {
  /// Settlement id.
  pub settlement_id: u32,
  /// Venue id.
  pub venue_id: u32,
  /// Legs.
  pub legs: String,
  /// Memo.
  pub memo: Option<String>,

  pub created_at: chrono::NaiveDateTime,
}

#[cfg(feature = "backend")]
impl SettlementRecord {
  pub fn from_tx(tx: &TransactionCreated) -> Result<Self> {
    Ok(Self {
      settlement_id: tx.transaction_id.0 as _,
      venue_id: tx.venue_id.0 as _,
      legs: serde_json::to_string(&tx.legs)?,
      memo: if tx.memo.len() > 0 {
        Some(tx.memo.clone())
      } else {
        None
      },
      ..Default::default()
    })
  }
}

/// Settlement event record.
#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SettlementEventRecord {
  /// Settlement id.
  pub settlement_id: u32,
  /// Settlement event.
  pub event: String,

  pub created_at: chrono::NaiveDateTime,
}

#[cfg(feature = "backend")]
impl SettlementEventRecord {
  pub fn from_events(processed_events: &ProcessedEvents) -> Result<Vec<Self>> {
    let mut events = Vec::new();
    for ev in &processed_events.0 {
      match ev {
        ProcessedEvent::ConfidentialTransactionCreated(TransactionCreated { transaction_id, .. }) |
        ProcessedEvent::ConfidentialTransactionAffirmed(TransactionAffirmed { transaction_id, .. }) |
        ProcessedEvent::ConfidentialTransactionRejected(transaction_id) |
        ProcessedEvent::ConfidentialTransactionExecuted(transaction_id) => {
          events.push(Self {
            settlement_id: transaction_id.0 as _,
            event: serde_json::to_string(ev)?,
            ..Default::default()
          })
        }
        _ => (),
      }
    }
    Ok(events)
  }
}

/// A Confidential asset transaction was created.
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct TransactionCreated {
  /// Confidential venue id.
  #[schema(value_type = u64)]
  pub venue_id: VenueId,
  /// Confidential transaction id.
  #[schema(value_type = u64)]
  pub transaction_id: TransactionId,
  /// Confidential transaction legs.
  pub legs: Vec<ConfidentialSettlementLeg>,
  /// Settlement memo.
  #[schema(example = "")]
  #[serde(default)]
  pub memo: String,
}

/// The transaction party affirmation.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub enum TransactionAffirmedParty {
  Sender,
  Receiver,
  Mediator(PublicKey),
}

/// A Confidential asset transaction was affirmed.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
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
  /// Who affirmed the transaction leg.
  pub party: TransactionAffirmedParty,
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
  /// A Confidential asset was created.
  ConfidentialAssetCreated(String),
  /// A Confidential asset minted.
  ///
  /// (ticker, amount minted, total_supply)
  ConfidentialAssetMinted(String, u64, u64),
  /// A Confidential asset Venue was created.
  #[schema(value_type = u64)]
  ConfidentialVenueCreated(VenueId),
  /// A Confidential asset transaction was created.
  ConfidentialTransactionCreated(TransactionCreated),
  /// A Confidential asset transaction executed.
  #[schema(value_type = u64)]
  ConfidentialTransactionExecuted(TransactionId),
  /// A Confidential asset transaction rejected.
  #[schema(value_type = u64)]
  ConfidentialTransactionRejected(TransactionId),
  /// A Confidential asset transaction was affirmed.
  ConfidentialTransactionAffirmed(TransactionAffirmed),
}

/// Processed events from the transaction.
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct ProcessedEvents(pub Vec<ProcessedEvent>);

impl ProcessedEvents {
  /// Get ids from *Created events.
  pub fn from_events(events: &[EventRecord<RuntimeEvent>]) -> Result<Self> {
    let mut processed = Vec::new();
    for rec in events {
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
        RuntimeEvent::ConfidentialAsset(ConfidentialAssetEvent::ConfidentialAssetCreated(_, ticker, ..)) => {
          let ticker = ticker_to_string(ticker);
          processed.push(ProcessedEvent::ConfidentialAssetCreated(ticker));
        }
        RuntimeEvent::ConfidentialAsset(ConfidentialAssetEvent::Issued(_, ticker, amount, total_supply)) => {
          let ticker = ticker_to_string(ticker);
          processed.push(ProcessedEvent::ConfidentialAssetMinted(ticker, *amount as _, *total_supply as _));
        }
        RuntimeEvent::ConfidentialAsset(ConfidentialAssetEvent::TransactionCreated(
          _,
          venue_id,
          id,
          legs,
          memo,
        )) => {
          let legs = legs.into_iter().map(|l| {
            let (mediators, auditors) = split_auditors(&l.auditors);
            ConfidentialSettlementLeg {
              ticker: ticker_to_string(&l.ticker),
              sender: PublicKey(l.sender.encode()),
              receiver: PublicKey(l.receiver.encode()),
              mediators,
              auditors,
            }
          }).collect();
          processed.push(ProcessedEvent::ConfidentialTransactionCreated(
              TransactionCreated {
                venue_id: *venue_id,
                transaction_id: *id,
                legs,
                memo: memo_to_string(memo),
              }
          ));
        }
        RuntimeEvent::ConfidentialAsset(ConfidentialAssetEvent::TransactionExecuted(
          _,
          id,
          ..,
        )) => {
          processed.push(ProcessedEvent::ConfidentialTransactionExecuted(*id));
        }
        RuntimeEvent::ConfidentialAsset(ConfidentialAssetEvent::TransactionRejected(
          _,
          id,
          ..,
        )) => {
          processed.push(ProcessedEvent::ConfidentialTransactionRejected(*id));
        }
        RuntimeEvent::ConfidentialAsset(ConfidentialAssetEvent::TransactionAffirmed(
          _,
          tx_id,
          leg_id,
          party,
          pending,
        )) => {
          match party {
            AffirmParty::Sender(sender_proof) => {
              processed.push(ProcessedEvent::ConfidentialTransactionAffirmed(
                TransactionAffirmed {
                  transaction_id: *tx_id,
                  pending_affirms: *pending,
                  leg_id: *leg_id,
                  sender_proof: Some(SenderProof(sender_proof.encode())),
                  party: TransactionAffirmedParty::Sender,
                },
              ));
            }
            AffirmParty::Receiver => {
              processed.push(ProcessedEvent::ConfidentialTransactionAffirmed(
                TransactionAffirmed {
                  transaction_id: *tx_id,
                  pending_affirms: *pending,
                  leg_id: *leg_id,
                  sender_proof: None,
                  party: TransactionAffirmedParty::Receiver,
                },
              ));
            }
            AffirmParty::Mediator(mediator) => {
              let mediator = PublicKey(mediator.encode());
              processed.push(ProcessedEvent::ConfidentialTransactionAffirmed(
                TransactionAffirmed {
                  transaction_id: *tx_id,
                  pending_affirms: *pending,
                  leg_id: *leg_id,
                  sender_proof: None,
                  party: TransactionAffirmedParty::Mediator(mediator),
                },
              ));
            }
          }
        }
        _ => (),
      }
    }
    Ok(Self(processed))
  }
}

/// Block transaction record.
#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct BlockTransactionRecord {
  /// Block hash.
  pub block_hash: String,
  /// Block number.
  pub block_number: u32,
  /// Transaction hash.
  pub tx_hash: String,
  /// Was the transaction sucessful.
  pub success: bool,
  /// If `success` is false, then provide an error message.
  pub error: Option<String>,
  /// Events.
  pub events: Option<String>,

  pub created_at: chrono::NaiveDateTime,
}

#[cfg(feature = "backend")]
impl BlockTransactionRecord {
  pub fn from_tx(tx: &TransactionResult) -> Result<Self> {
    Ok(Self {
      block_hash: tx.block_hash.clone(),
      block_number: tx.block_number,
      tx_hash: tx.tx_hash.clone(),
      events: if tx.processed_events.0.len() > 0 {
        Some(serde_json::to_string(&tx.processed_events)?)
      } else {
        None
      },
      success: tx.success,
      error: tx.err_msg.clone(),
      ..Default::default()
    })
  }
}

/// Transaction results
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct TransactionResult {
  /// Block hash.
  #[schema(example = "0xea549dcdadacb5678e37a336e44c581ade562b696159bf8fd846fee7e7fe1dc3")]
  pub block_hash: String,
  /// Block number.
  #[schema(example = 1)]
  pub block_number: u32,
  /// Transaction hash.
  #[schema(example = "0xea549dcdadacb5678e37a336e44c581ade562b696159bf8fd846fee7e7fe1dc3")]
  pub tx_hash: String,
  /// Was the transaction sucessful.
  #[schema(example = true)]
  pub success: bool,
  /// If `success` is false, then provide an error message.
  #[schema(example = json!(null))]
  pub err_msg: Option<String>,
  /// Processed Events.
  #[schema(example = json!([]))]
  pub processed_events: ProcessedEvents,
}

#[cfg(feature = "backend")]
impl TransactionResult {
  pub async fn get_block_transactions(api: &Api, header: Header) -> Result<Vec<Self>> {
    let block_hash = header.hash();
    let block_events = api.block_events(Some(block_hash)).await?;
    let block = api.client().get_block(Some(block_hash)).await?;

    let mut transactions = Vec::new();
    if let Some(block) = block {
      let block_hash = format!("{block_hash:#x}");
      for (idx, tx_enc) in block.extrinsics().into_iter().enumerate() {
        let tx_hash = ExtrinsicV4::tx_hash(&tx_enc.0);
        let events = block_events.iter().filter(|ev| ev.phase == Phase::ApplyExtrinsic(idx as u32))
          .cloned()
          .collect::<Vec<_>>();
        let tx_res = Api::events_to_extrinsic_result(&events);
        let (success, err_msg) = match tx_res {
          Some(ExtrinsicResult::Success(_)) => (true, None),
          Some(ExtrinsicResult::Failed(_, err)) => (false, Some(format!("{:?}", err.as_short_doc()))),
          None => (false, Some(format!("Unknown transaction results"))),
        };
        transactions.push(Self {
          block_hash: block_hash.clone(),
          block_number: header.number,
          tx_hash: format!("{:#x}", tx_hash),
          success,
          err_msg,
          processed_events: ProcessedEvents::from_events(&events)?,
        })
      }
    }
    Ok(transactions)
  }

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
    res.tx_hash = format!("{:#x}", tx_res.hash());

    if let Some(header) = tx_res.get_block_header().await? {
      res.block_number = header.number;
    }

    // Process events.
    if let Some(events) = tx_res.events().await? {
      res.processed_events = ProcessedEvents::from_events(&events.0)?;
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

pub fn ticker_to_string(ticker: &Ticker) -> String {
  // Truncate at first null.
  if let Some(t) = ticker.0.split(|&c| c == 0).next() {
    String::from_utf8_lossy(t).to_string()
  } else {
    "".to_string()
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

pub fn memo_to_string(memo: &Option<Memo>) -> String {
  match memo {
    Some(memo) => {
      format!("0x{}", hex::encode(&memo.0[..]))
    }
    None => "".to_string(),
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
  /// List of mediators.
  #[schema(example = json!([]))]
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
  #[schema(example = json!([]))]
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
    join_auditors(&self.mediators, &self.auditors)
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
  #[schema(example = json!([]))]
  #[serde(default)]
  pub mediators: Vec<PublicKey>,
  /// List of auditors.
  #[schema(example = json!(["0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114"]))]
  #[serde(default)]
  pub auditors: Vec<PublicKey>,
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
    join_auditors(&self.mediators, &self.auditors)
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
