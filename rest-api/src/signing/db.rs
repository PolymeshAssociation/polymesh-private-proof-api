use std::sync::Arc;

use actix_web::web::Data;

use async_trait::async_trait;
use confidential_proof_shared::{error::Result, SignerInfo, SignerWithSecret, CreateSigner};

use polymesh_api::client::PairSigner;

use super::{AppSigningManager, SigningManagerTrait, TxSigner};

pub struct SqliteSigningManager {
  pool: sqlx::SqlitePool,
}

impl SqliteSigningManager {
  pub fn new(pool: &sqlx::SqlitePool) -> Arc<dyn SigningManagerTrait> {
    Arc::new(Self { pool: pool.clone() })
  }

  pub fn new_app_data(pool: &sqlx::SqlitePool) -> AppSigningManager {
    Data::from(Self::new(pool))
  }
}

#[async_trait]
impl SigningManagerTrait for SqliteSigningManager {
  async fn get_signers(&self) -> Result<Vec<SignerInfo>> {
    Ok(
      sqlx::query_as!(
        SignerInfo,
        r#"SELECT signer_name as name, public_key, created_at FROM signers"#,
      )
      .fetch_all(&self.pool)
      .await?,
    )
  }

  async fn get_signer_info(&self, signer: &str) -> Result<Option<SignerInfo>> {
    Ok(
      sqlx::query_as!(
        SignerInfo,
        r#"SELECT signer_name as name, public_key, created_at
        FROM signers WHERE signer_name = ?"#,
        signer
      )
      .fetch_optional(&self.pool)
      .await?,
    )
  }

  async fn get_signer(&self, signer: &str) -> Result<Option<TxSigner>> {
    let signer = sqlx::query_as!(
        SignerWithSecret,
        r#"SELECT signer_name as name, public_key, secret_key
        FROM signers WHERE signer_name = ?"#,
        signer
      )
      .fetch_optional(&self.pool)
      .await?;
    match signer {
      Some(signer) => {
        let signer = PairSigner::new(signer.keypair()?);
        Ok(Some(Box::new(signer)))
      }
      None => Ok(None),
    }
  }

  async fn create_signer(&self, signer: &CreateSigner) -> Result<SignerInfo> {
    let signer = signer.as_signer_with_secret()?;
    Ok(
      sqlx::query_as!(
        SignerInfo,
        r#"
      INSERT INTO signers (signer_name, public_key, secret_key)
      VALUES (?, ?, ?)
      RETURNING signer_name as name, public_key, created_at
      "#,
        signer.name,
        signer.public_key,
        signer.secret_key,
      )
      .fetch_one(&self.pool)
      .await?,
    )
  }
}
