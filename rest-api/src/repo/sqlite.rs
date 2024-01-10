use std::sync::Arc;

use actix_web::web::Data;

use async_trait::async_trait;
use confidential_proof_shared::{
  error::Result, BlockTransactionRecord, SettlementEventRecord, SettlementRecord,
};

use super::{TransactionRepository, TransactionRepositoryTrait};

pub struct SqliteTransactionRepository {
  pool: sqlx::SqlitePool,
}

impl SqliteTransactionRepository {
  pub fn new(pool: &sqlx::SqlitePool) -> Arc<dyn TransactionRepositoryTrait> {
    Arc::new(Self { pool: pool.clone() })
  }

  pub fn new_app_data(pool: &sqlx::SqlitePool) -> TransactionRepository {
    Data::from(Self::new(pool))
  }
}

#[async_trait]
impl TransactionRepositoryTrait for SqliteTransactionRepository {
  // Block transactions.
  async fn get_block_transactions(&self) -> Result<Vec<BlockTransactionRecord>> {
    Ok(
      sqlx::query_as!(BlockTransactionRecord, r#"
        SELECT block_hash, block_number as "block_number: u32", tx_hash, success as "success: bool", error, events, created_at
        FROM transactions
        "#,)
        .fetch_all(&self.pool)
        .await?,
    )
  }

  async fn get_block_transaction(&self, tx_hash: &[u8]) -> Result<Option<BlockTransactionRecord>> {
    Ok(
      sqlx::query_as!(BlockTransactionRecord, r#"
        SELECT block_hash, block_number as "block_number: u32", tx_hash, success as "success: bool", error, events, created_at
        FROM transactions
        WHERE tx_hash = ?
        "#, tx_hash)
        .fetch_optional(&self.pool)
        .await?,
    )
  }

  async fn add_block_transaction(&self, tx: BlockTransactionRecord) -> Result<()> {
    sqlx::query!(
      r#"
      INSERT INTO transactions (block_hash, block_number, tx_hash, success, error, events)
      VALUES (?, ?, ?, ?, ?, ?)
      "#,
      tx.block_hash,
      tx.block_number,
      tx.tx_hash,
      tx.success,
      tx.error,
      tx.events,
    )
    .execute(&self.pool)
    .await?;
    Ok(())
  }

  // Settlements.
  async fn get_settlements(&self) -> Result<Vec<SettlementRecord>> {
    Ok(
      sqlx::query_as!(SettlementRecord, r#"
        SELECT settlement_id as "settlement_id: u32", venue_id as "venue_id: u32", legs, memo, created_at
        FROM settlements
        "#,)
        .fetch_all(&self.pool)
        .await?,
    )
  }

  async fn get_settlement(&self, settlement_id: i64) -> Result<Option<SettlementRecord>> {
    Ok(
      sqlx::query_as!(SettlementRecord, r#"
        SELECT settlement_id as "settlement_id: u32", venue_id as "venue_id: u32", legs, memo, created_at
        FROM settlements
        WHERE settlement_id = ?
        "#, settlement_id)
        .fetch_optional(&self.pool)
        .await?,
    )
  }

  async fn add_settlement(&self, rec: SettlementRecord) -> Result<()> {
    sqlx::query!(
      r#"
      INSERT INTO settlements (settlement_id, venue_id, legs, memo)
      VALUES (?, ?, ?, ?)
      "#,
      rec.settlement_id,
      rec.venue_id,
      rec.legs,
      rec.memo,
    )
    .execute(&self.pool)
    .await?;
    Ok(())
  }

  // Settlement Events.
  async fn get_settlement_events(&self, settlement_id: i64) -> Result<Vec<SettlementEventRecord>> {
    Ok(
      sqlx::query_as!(
        SettlementEventRecord,
        r#"
        SELECT settlement_id as "settlement_id: u32", event, created_at
        FROM settlement_events
        WHERE settlement_id = ?
        "#,
        settlement_id
      )
      .fetch_all(&self.pool)
      .await?,
    )
  }

  async fn add_settlement_event(&self, rec: SettlementEventRecord) -> Result<()> {
    sqlx::query!(
      r#"
      INSERT INTO settlement_events (settlement_id, event)
      VALUES (?, ?)
      "#,
      rec.settlement_id,
      rec.event,
    )
    .execute(&self.pool)
    .await?;
    Ok(())
  }
}
