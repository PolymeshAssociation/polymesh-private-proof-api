use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

use serde::{Deserialize, Serialize};
use serde_hex::{SerHex, StrictPfx};

use utoipa::ToSchema;

use codec::{Decode, Encode};

#[cfg(feature = "backend")]
use polymesh_api::{
  client::{
    basic_types::{AccountId, IdentityId},
    block::{EventRecord, ExtrinsicV4, Header, Phase},
    EnumInfo, ExtrinsicResult,
  },
  types::{
    pallet_confidential_asset::{
      AffirmParty, AuditorAccount, ConfidentialAccount, ConfidentialAuditors, TransactionId,
      TransactionLeg, TransactionLegId,
    },
    polymesh_common_utilities::traits::checkpoint::ScheduleId,
    polymesh_primitives::{
      asset::CheckpointId,
      settlement::{InstructionId, VenueId},
      Memo,
    },
    runtime::{events::*, RuntimeEvent},
  },
  Api, ChainApi, TransactionResults,
};

#[cfg(feature = "backend")]
use confidential_assets::{Balance, CipherText, ElgamalPublicKey};

use crate::error::Result;
use crate::proofs::{
  AccountWithSecret, PublicKey, SenderProof, TransferProofs, UpdateAccountAsset,
};

pub fn scale_convert<T1: Encode, T2: Decode>(t1: &T1) -> T2 {
  let buf = t1.encode();
  T2::decode(&mut &buf[..]).expect("The two types don't have compatible SCALE encoding")
}

pub fn confidential_account_to_key(account: &ConfidentialAccount) -> ElgamalPublicKey {
  scale_convert(account)
}

pub fn auditor_account_to_key(account: &AuditorAccount) -> ElgamalPublicKey {
  scale_convert(account)
}

pub fn join_auditors(
  mediators: &[IdentityId],
  auditors: &[PublicKey],
) -> Result<ConfidentialAuditors> {
  Ok(ConfidentialAuditors {
    auditors: auditors
      .iter()
      .map(|k| k.as_auditor_account())
      .collect::<Result<BTreeSet<_>>>()?,
    mediators: mediators.iter().map(|m| m.clone()).collect(),
  })
}

pub fn split_auditors(auditors: &ConfidentialAuditors) -> (Vec<IdentityId>, Vec<PublicKey>) {
  let mediators = auditors.mediators.iter().map(|m| m.clone()).collect();
  let auditors = auditors.auditors.iter().map(|k| scale_convert(k)).collect();
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
        ProcessedEvent::ConfidentialTransactionCreated(TransactionCreated {
          transaction_id,
          ..
        })
        | ProcessedEvent::ConfidentialTransactionAffirmed(TransactionAffirmed {
          transaction_id,
          ..
        })
        | ProcessedEvent::ConfidentialTransactionRejected { transaction_id }
        | ProcessedEvent::ConfidentialTransactionExecuted { transaction_id } => events.push(Self {
          settlement_id: transaction_id.0 as _,
          event: serde_json::to_string(ev)?,
          ..Default::default()
        }),
        _ => (),
      }
    }
    Ok(events)
  }
}

/// Confidential asset transaction leg details.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct TransactionLegDetails {
  /// Asset ids and auditors for each asset.
  #[schema(example = json!({"3480b2c3-221f-de22-226f-a178e13ff916": ["0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114"]}))]
  #[serde(default)]
  pub assets_and_auditors: BTreeMap<Uuid, BTreeSet<PublicKey>>,
  /// Sender's public key.
  #[schema(value_type = String, format = Binary, example = "0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114")]
  pub sender: PublicKey,
  /// Receiver's public key.
  #[schema(value_type = String, format = Binary, example = "0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114")]
  pub receiver: PublicKey,
  /// Set of mediator identities for this leg.
  #[schema(example = json!([]))]
  #[serde(default)]
  pub mediators: BTreeSet<IdentityId>,
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
  /// Confidential transaction leg details.
  pub legs: Vec<TransactionLegDetails>,
  /// Settlement memo.
  #[schema(example = "")]
  #[serde(default)]
  pub memo: String,
}

/// The transaction party.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub enum TransactionParty {
  #[default]
  Sender,
  Receiver,
  Mediator,
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
  /// Confidential transaction leg transfer proofs (if the sender affirmed).
  pub transfer_proofs: Option<TransferProofs>,
  /// Who affirmed the transaction leg.
  pub party: TransactionParty,
}

/// Type of balance update.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, ToSchema)]
pub enum BalanceUpdateAction {
  #[default]
  Withdraw,
  Deposit,
  DepositIncoming,
}

/// A Confidential account's balance has updated.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct BalanceUpdated {
  /// Confidential account.
  #[schema(value_type = String, format = Binary, example = "0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114")]
  pub account: PublicKey,
  /// Asset id.
  pub asset_id: Uuid,
  /// The update action.
  pub action: BalanceUpdateAction,
  /// Encrypted amount.
  #[schema(value_type = String, format = Binary, example = "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")]
  #[serde(with = "SerHex::<StrictPfx>")]
  pub amount: [u8; 64],
  /// New Encrypted balance.
  #[schema(value_type = String, format = Binary, example = "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")]
  #[serde(with = "SerHex::<StrictPfx>")]
  pub balance: [u8; 64],
}

impl BalanceUpdated {
  pub fn amount(&self) -> Result<CipherText> {
    Ok(CipherText::decode(&mut self.amount.as_slice())?)
  }

  pub fn balance(&self) -> Result<CipherText> {
    Ok(CipherText::decode(&mut self.balance.as_slice())?)
  }

  pub fn try_decrypt(&self, account: &AccountWithSecret) -> Option<AccountAssetBalanceUpdated> {
    // Only try decrypting our updates.
    if !account.match_public_key(&self.account) {
      return None;
    }
    let amount = self.amount().ok()?;
    let balance = self.balance().ok()?;
    Some(AccountAssetBalanceUpdated {
      asset_id: self.asset_id,
      action: self.action,
      amount: account.decrypt(&amount).ok()?,
      balance: account.decrypt(&balance).ok()?,
    })
  }
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
  ConfidentialAssetCreated { asset_id: Uuid },
  /// A Confidential asset minted.
  ///
  /// (asset_id, amount minted, total_supply)
  ConfidentialAssetMinted {
    asset_id: Uuid,
    amount: u64,
    total_supply: u64,
  },
  /// A Confidential asset Venue was created.
  ConfidentialVenueCreated {
    #[schema(value_type = u64)]
    venue_id: VenueId,
  },
  /// A Confidential account's balance was updated.
  ConfidentialAccountBalanceUpdated(BalanceUpdated),
  /// A Confidential asset transaction was created.
  ConfidentialTransactionCreated(TransactionCreated),
  /// A Confidential asset transaction executed.
  ConfidentialTransactionExecuted {
    #[schema(value_type = u64)]
    transaction_id: TransactionId,
  },
  /// A Confidential asset transaction rejected.
  ConfidentialTransactionRejected {
    #[schema(value_type = u64)]
    transaction_id: TransactionId,
  },
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
        RuntimeEvent::ConfidentialAsset(ConfidentialAssetEvent::VenueCreated{ venue_id, .. }) => {
          processed.push(ProcessedEvent::ConfidentialVenueCreated { venue_id: *venue_id });
        }
        RuntimeEvent::ConfidentialAsset(ConfidentialAssetEvent::AssetCreated{
          asset_id,
          ..
        }) => {
          processed.push(ProcessedEvent::ConfidentialAssetCreated {
            asset_id: Uuid::from_bytes(*asset_id),
          });
        }
        RuntimeEvent::ConfidentialAsset(ConfidentialAssetEvent::Issued{
          asset_id,
          amount,
          total_supply,
          ..
        }) => {
          processed.push(ProcessedEvent::ConfidentialAssetMinted {
            asset_id: Uuid::from_bytes(*asset_id),
            amount: *amount as _,
            total_supply: *total_supply as _,
          });
        }
        RuntimeEvent::ConfidentialAsset(ConfidentialAssetEvent::AccountDeposit{
          account,
          asset_id,
          amount,
          balance,
        }) => {
          processed.push(ProcessedEvent::ConfidentialAccountBalanceUpdated(
            BalanceUpdated {
              account: scale_convert(account),
              asset_id: Uuid::from_bytes(*asset_id),
              action: BalanceUpdateAction::Deposit,
              amount: scale_convert(amount),
              balance: scale_convert(balance),
            },
          ));
        }
        RuntimeEvent::ConfidentialAsset(ConfidentialAssetEvent::AccountDepositIncoming{
          account,
          asset_id,
          amount,
          incoming_balance,
        }) => {
          processed.push(ProcessedEvent::ConfidentialAccountBalanceUpdated(
            BalanceUpdated {
              account: scale_convert(account),
              asset_id: Uuid::from_bytes(*asset_id),
              action: BalanceUpdateAction::DepositIncoming,
              amount: scale_convert(amount),
              balance: scale_convert(incoming_balance),
            },
          ));
        }
        RuntimeEvent::ConfidentialAsset(ConfidentialAssetEvent::AccountWithdraw{
          account,
          asset_id,
          amount,
          balance,
        }) => {
          processed.push(ProcessedEvent::ConfidentialAccountBalanceUpdated(
            BalanceUpdated {
              account: scale_convert(account),
              asset_id: Uuid::from_bytes(*asset_id),
              action: BalanceUpdateAction::Withdraw,
              amount: scale_convert(amount),
              balance: scale_convert(balance),
            },
          ));
        }
        RuntimeEvent::ConfidentialAsset(ConfidentialAssetEvent::TransactionCreated{
          venue_id,
          transaction_id,
          legs,
          memo,
          ..
        }) => {
          let legs = legs
            .into_iter()
            .map(|l| TransactionLegDetails {
              assets_and_auditors: l
                .auditors
                .iter()
                .map(|(id, keys)| {
                  (
                    Uuid::from_bytes(*id),
                    keys.iter().map(|k| scale_convert(k)).collect(),
                  )
                })
                .collect(),
              sender: scale_convert(&l.sender),
              receiver: scale_convert(&l.receiver),
              mediators: l.mediators.clone().into(),
            })
            .collect();
          processed.push(ProcessedEvent::ConfidentialTransactionCreated(
            TransactionCreated {
              venue_id: *venue_id,
              transaction_id: *transaction_id,
              legs,
              memo: memo_to_string(memo),
            },
          ));
        }
        RuntimeEvent::ConfidentialAsset(ConfidentialAssetEvent::TransactionExecuted{transaction_id, ..}) => {
          processed.push(ProcessedEvent::ConfidentialTransactionExecuted {
            transaction_id: *transaction_id,
          });
        }
        RuntimeEvent::ConfidentialAsset(ConfidentialAssetEvent::TransactionRejected{transaction_id, ..}) => {
          processed.push(ProcessedEvent::ConfidentialTransactionRejected {
            transaction_id: *transaction_id,
          });
        }
        RuntimeEvent::ConfidentialAsset(ConfidentialAssetEvent::TransactionAffirmed{
          transaction_id,
          leg_id,
          party,
          pending_affirms,
          ..
        }) => match party {
          AffirmParty::Sender(transfers) => {
            let transfers = TransferProofs {
              proofs: transfers
                .proofs
                .iter()
                .map(|(asset_id, proof)| {
                  (Uuid::from_bytes(*asset_id), SenderProof(proof.0.clone()))
                })
                .collect(),
            };
            processed.push(ProcessedEvent::ConfidentialTransactionAffirmed(
              TransactionAffirmed {
                transaction_id: *transaction_id,
                pending_affirms: *pending_affirms,
                leg_id: *leg_id,
                transfer_proofs: Some(transfers),
                party: TransactionParty::Sender,
              },
            ));
          }
          AffirmParty::Receiver => {
            processed.push(ProcessedEvent::ConfidentialTransactionAffirmed(
              TransactionAffirmed {
                transaction_id: *transaction_id,
                pending_affirms: *pending_affirms,
                leg_id: *leg_id,
                transfer_proofs: None,
                party: TransactionParty::Receiver,
              },
            ));
          }
          AffirmParty::Mediator => {
            processed.push(ProcessedEvent::ConfidentialTransactionAffirmed(
              TransactionAffirmed {
                transaction_id: *transaction_id,
                pending_affirms: *pending_affirms,
                leg_id: *leg_id,
                transfer_proofs: None,
                party: TransactionParty::Mediator,
              },
            ));
          }
        },
        _ => (),
      }
    }
    Ok(Self(processed))
  }
}

/// Account asset incoming balance.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AccountAssetIncomingBalance {
  /// Asset id.
  pub asset_id: Uuid,
  /// Decrypted incoming amount.
  #[schema(example = 1000, value_type = u64)]
  pub incoming_amount: Balance,
}

/// Account asset balance updated.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AccountAssetBalanceUpdated {
  /// Asset id.
  pub asset_id: Uuid,
  /// The update action.
  pub action: BalanceUpdateAction,
  /// Decrypted amount.
  #[schema(example = 1000, value_type = u64)]
  pub amount: Balance,
  /// Decrypted new balance.
  #[schema(example = 1000, value_type = u64)]
  pub balance: Balance,
}

/// Account asset balances updated.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AccountAssetBalancesUpdated {
  pub updates: Vec<AccountAssetBalanceUpdated>,
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
  /// Account balances updated.
  #[schema(example = json!([]))]
  pub balances_updated: Option<AccountAssetBalancesUpdated>,
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
        let events = block_events
          .iter()
          .filter(|ev| ev.phase == Phase::ApplyExtrinsic(idx as u32))
          .cloned()
          .collect::<Vec<_>>();
        let tx_res = Api::events_to_extrinsic_result(&events);
        let (success, err_msg) = match tx_res {
          Some(ExtrinsicResult::Success(_)) => (true, None),
          Some(ExtrinsicResult::Failed(_, err)) => {
            (false, Some(format!("{:?}", err.as_short_doc())))
          }
          None => (false, Some(format!("Unknown transaction results"))),
        };
        transactions.push(Self {
          block_hash: block_hash.clone(),
          block_number: header.number,
          tx_hash: format!("{:#x}", tx_hash),
          success,
          err_msg,
          processed_events: ProcessedEvents::from_events(&events)?,
          balances_updated: None,
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

  pub fn decrypt_balance_updates(
    &mut self,
    account: &AccountWithSecret,
  ) -> Option<BTreeMap<Uuid, UpdateAccountAsset>> {
    let mut asset_updates = BTreeMap::new();
    let mut updates = Vec::new();
    for event in &self.processed_events.0 {
      match event {
        ProcessedEvent::ConfidentialAccountBalanceUpdated(balance_updated) => {
          if let Some(update) = balance_updated.try_decrypt(account) {
            asset_updates.insert(
              update.asset_id,
              UpdateAccountAsset {
                account_asset_id: None,
                account_id: account.account_id,
                asset_id: update.asset_id,
                balance: update.balance,
                enc_balance: balance_updated.balance().ok()?,
              },
            );
            updates.push(update);
          }
        }
        _ => (),
      }
    }
    if updates.len() > 0 {
      self.balances_updated = Some(AccountAssetBalancesUpdated { updates });
      Some(asset_updates)
    } else {
      None
    }
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

/// Confidential asset details (name, auditors).
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct ConfidentialAssetDetails {
  /// Asset total supply.
  #[schema(example = "10000")]
  pub total_supply: u64,
  /// Asset owner.
  #[schema(example = json!(IdentityId::default()))]
  pub owner: IdentityId,
  /// List of mediator identities.
  #[schema(example = json!([]))]
  #[serde(default)]
  pub mediators: Vec<IdentityId>,
  /// List of auditor Elgamal public keys.
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
  /// List of mediators identities.
  #[schema(example = json!([]))]
  #[serde(default)]
  pub mediators: Vec<IdentityId>,
  /// List of auditor Elgamal public key.
  #[schema(example = json!(["0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114"]))]
  #[serde(default)]
  pub auditors: Vec<PublicKey>,
}

#[cfg(feature = "backend")]
impl CreateConfidentialAsset {
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
  /// Asset id.
  pub assets: BTreeSet<Uuid>,
  /// Sender's public key.
  #[schema(value_type = String, format = Binary, example = "0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114")]
  pub sender: PublicKey,
  /// Receiver's public key.
  #[schema(value_type = String, format = Binary, example = "0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114")]
  pub receiver: PublicKey,
  /// Set of venue mediator identities for this leg.
  #[schema(example = json!([]))]
  #[serde(default)]
  pub mediators: BTreeSet<IdentityId>,
  /// Set of venue auditor Elgamal public keys for this leg.
  #[schema(example = json!(["0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114"]))]
  #[serde(default)]
  pub auditors: BTreeSet<PublicKey>,
}

#[cfg(feature = "backend")]
impl ConfidentialSettlementLeg {
  pub fn sender(&self) -> Result<ConfidentialAccount> {
    Ok(self.sender.as_confidential_account()?)
  }

  pub fn receiver(&self) -> Result<ConfidentialAccount> {
    Ok(self.receiver.as_confidential_account()?)
  }

  pub fn auditors(&self) -> Result<BTreeSet<AuditorAccount>> {
    self
      .auditors
      .iter()
      .map(|k| k.as_auditor_account())
      .collect()
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
        assets: leg.assets.iter().map(|id| *id.as_bytes()).collect(),
        sender: leg.sender()?,
        receiver: leg.receiver()?,
        auditors: leg.auditors()?,
        mediators: leg.mediators.iter().map(|m| m.clone()).collect(),
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

/// Asset id and amount.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct TransactionAssetAmount {
  /// Asset id.
  pub asset_id: Uuid,
  /// The asset amount.
  #[schema(example = 1000, value_type = u64)]
  pub amount: Balance,
}

/// Affirm Confidential asset transaction leg as the sender/receiver/mediator.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct AffirmTransactionLeg {
  /// Confidential transaction leg id.
  #[schema(value_type = u32)]
  pub leg_id: TransactionLegId,
  /// The amount for each asset in the leg.
  pub amounts: Option<Vec<TransactionAssetAmount>>,
  /// Who is affirming the transaction.
  pub party: TransactionParty,
}

/// Affirm Confidential asset transaction as the sender/receiver/mediator.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct AffirmTransactionRequest {
  /// Confidential transaction id.
  #[schema(value_type = u64)]
  pub transaction_id: TransactionId,
  /// Confidential transaction legs to affirm.
  pub legs: Vec<AffirmTransactionLeg>,
}

/// Affirm Confidential asset transactions as the sender/receiver/mediator.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct AffirmTransactionsRequest {
  /// Signer of the transaction.
  #[schema(example = "Alice")]
  pub signer: String,
  /// Wait for block finalization.
  #[schema(example = false)]
  #[serde(default)]
  pub finalize: bool,
  /// Confidential transactions to affirm.
  pub transactions: Vec<AffirmTransactionRequest>,
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
