use actix_web::web::Data;

use async_trait::async_trait;
use confidential_proof_shared::{
  error::Result, BlockTransactionRecord, SettlementEventRecord, SettlementRecord,
};

mod sqlite;

pub use sqlite::SqliteTransactionRepository;

pub type TransactionRepository = Data<dyn TransactionRepositoryTrait>;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TransactionRepositoryTrait: Send + Sync + 'static {
  // Block transactions.
  async fn get_block_transactions(&self) -> Result<Vec<BlockTransactionRecord>>;
  async fn get_block_transaction(&self, tx_hash: &[u8]) -> Result<Option<BlockTransactionRecord>>;
  async fn add_block_transaction(&self, rec: BlockTransactionRecord) -> Result<()>;

  // Settlements.
  async fn get_settlements(&self) -> Result<Vec<SettlementRecord>>;
  async fn get_settlement(&self, settlement_id: i64) -> Result<Option<SettlementRecord>>;
  async fn add_settlement(&self, rec: SettlementRecord) -> Result<()>;

  // Settlement Events.
  async fn get_settlement_events(&self, settlement_id: i64) -> Result<Vec<SettlementEventRecord>>;
  async fn add_settlement_event(&self, rec: SettlementEventRecord) -> Result<()>;
}
