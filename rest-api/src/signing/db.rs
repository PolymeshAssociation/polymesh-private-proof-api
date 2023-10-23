use async_trait::async_trait;
use confidential_proof_shared::{
  error::Result, Signer, SignerWithSecret,
};

use super::{SigningManagerTrait, SigningManager};

pub struct SqliteSigningManager {
  pool: sqlx::SqlitePool,
}

impl SqliteSigningManager {
  pub fn new(pool: &sqlx::SqlitePool) -> SigningManager {
    Box::new(Self { pool: pool.clone() })
  }
}

#[async_trait]
impl SigningManagerTrait for SqliteSigningManager {
  async fn get_signers(&self) -> Result<Vec<Signer>> {
    Ok(
      sqlx::query_as!(
        Signer,
        r#"SELECT signer_id, signer_name, public_key, created_at, updated_at FROM signers"#,
      )
      .fetch_all(&self.pool)
      .await?,
    )
  }

  async fn get_signer(&self, signer: &str) -> Result<Option<Signer>> {
    Ok(
      sqlx::query_as!(
        Signer,
        r#"SELECT signer_id, signer_name, public_key, created_at, updated_at
        FROM signers WHERE signer_name = ?"#,
        signer
      )
      .fetch_optional(&self.pool)
      .await?,
    )
  }

  async fn get_signer_with_secret(&self, signer: &str) -> Result<Option<SignerWithSecret>> {
    Ok(
      sqlx::query_as!(
        SignerWithSecret,
        r#"SELECT signer_id, signer_name, public_key, secret_key
        FROM signers WHERE signer_name = ?"#,
        signer
      )
      .fetch_optional(&self.pool)
      .await?,
    )
  }

  async fn create_signer(&self, signer: &SignerWithSecret) -> Result<Signer> {
    Ok(
      sqlx::query_as!(
        Signer,
        r#"
      INSERT INTO signers (signer_name, public_key, secret_key)
      VALUES (?, ?, ?)
      RETURNING signer_id, signer_name, public_key, created_at, updated_at
      "#,
        signer.signer_name,
        signer.public_key,
        signer.secret_key,
      )
      .fetch_one(&self.pool)
      .await?,
    )
  }
}
