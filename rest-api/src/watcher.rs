use polymesh_api::*;

use confidential_proof_api::repo::Repository;
use confidential_proof_shared::*;

use crate::repo::TransactionRepository;

pub async fn start_chain_watcher(api: Api, repo: Repository, tx_repo: TransactionRepository) -> anyhow::Result<()> {
  let client = api.client();

  let mut sub_blocks = client.subscribe_blocks().await?;

  while let Some(header) = sub_blocks.next().await.transpose()? {
    let transactions = TransactionResult::get_block_transactions(&api, header).await?;
    if transactions.len() > 1 {
      for tx in transactions {
        let rec = BlockTransactionRecord::from_tx(&tx)?;
        // Add block transaction record.
        tx_repo.add_block_transaction(rec).await?;
        // process events.
        for ev in &tx.processed_events.0 {
          match ev {
            ProcessedEvent::ConfidentialTransactionCreated(created) => {
              let rec = SettlementRecord::from_tx(created)?;
              tx_repo.add_settlement(rec).await?;
            }
            ProcessedEvent::ConfidentialAssetCreated(ticker) => {
              // Check if the asset exists.
              if repo.get_asset(&ticker).await?.is_none() {
                repo.create_asset(&CreateAsset { ticker: ticker.clone() }).await?;
              }
            }
            _ => (),
          }
        }
        // Settlement events.
        let recs = SettlementEventRecord::from_events(&tx.processed_events)?;
        for rec in recs {
          tx_repo.add_settlement_event(rec).await?;
        }
      }
    }
  }

  Ok(())
}
