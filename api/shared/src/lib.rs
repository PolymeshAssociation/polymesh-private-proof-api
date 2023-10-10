use serde::{Deserialize, Serialize};
use serde_hex::{SerHexSeq, StrictPfx};

use zeroize::{Zeroize, ZeroizeOnDrop};

#[cfg(feature = "backend")]
use codec::{Decode, Encode};

#[cfg(feature = "backend")]
use std::collections::BTreeMap;

#[cfg(feature = "backend")]
use confidential_assets::{
  elgamal::CipherText,
  transaction::{AuditorId, ConfidentialTransferProof},
  Balance, ElgamalKeys, ElgamalPublicKey, ElgamalSecretKey, Scalar,
};

#[cfg(not(feature = "backend"))]
pub type Balance = u64;

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct User {
  pub user_id: i64,
  pub username: String,

  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CreateUser {
  pub username: String,
}

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Asset {
  pub asset_id: i64,
  pub ticker: String,

  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CreateAsset {
  pub ticker: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Account {
  pub account_id: i64,

  #[serde(with = "SerHexSeq::<StrictPfx>")]
  pub public_key: Vec<u8>,

  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Zeroize, ZeroizeOnDrop)]
#[cfg(feature = "backend")]
pub struct AccountWithSecret {
  pub account_id: i64,

  pub public_key: Vec<u8>,
  pub secret_key: Vec<u8>,
}

#[cfg(feature = "backend")]
impl AccountWithSecret {
  pub fn encryption_keys(&self) -> Option<ElgamalKeys> {
    Some(ElgamalKeys {
      public: ElgamalPublicKey::decode(&mut self.public_key.as_slice()).ok()?,
      secret: ElgamalSecretKey::decode(&mut self.secret_key.as_slice()).ok()?,
    })
  }

  pub fn init_balance(&self, asset_id: i64) -> UpdateAccountAsset {
    UpdateAccountAsset {
      account_id: self.account_id,
      asset_id,
      balance: 0,
      enc_balance: CipherText::zero(),
    }
  }

  pub fn auditor_verify_tx(&self, req: &AuditorVerifyRequest) -> Result<bool, String> {
    // Decode MercatAccount from database.
    let auditor = self
      .encryption_keys()
      .ok_or_else(|| format!("Failed to get account from database."))?;

    // Decode request.
    let sender_proof = req.sender_proof()?;

    let amount = sender_proof
      .auditor_verify(AuditorId(0), &auditor)
      .map_err(|e| format!("Failed to verify sender proof: {e:?}"))?;
    if amount != req.amount {
      return Err(format!("Failed to verify sender proof: Invalid transaction amount").into());
    }
    Ok(true)
  }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, Zeroize, ZeroizeOnDrop)]
pub struct CreateAccount {
  #[serde(with = "SerHexSeq::<StrictPfx>")]
  pub public_key: Vec<u8>,
  #[serde(skip)]
  pub secret_key: Vec<u8>,
}

#[cfg(feature = "backend")]
impl CreateAccount {
  fn create_secret_account() -> ElgamalKeys {
    let mut rng = rand::thread_rng();
    let secret = ElgamalSecretKey::new(Scalar::random(&mut rng));
    let public = secret.get_public_key();
    ElgamalKeys { public, secret }
  }

  pub fn new() -> Self {
    let enc_keys = Self::create_secret_account();

    Self {
      public_key: enc_keys.public.encode(),
      secret_key: enc_keys.secret.encode(),
    }
  }
}

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AccountAsset {
  pub account_asset_id: i64,
  pub account_id: i64,
  pub asset_id: i64,

  pub balance: i64,
  #[serde(with = "SerHexSeq::<StrictPfx>")]
  pub enc_balance: Vec<u8>,

  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[cfg(feature = "backend")]
impl AccountAsset {
  pub fn enc_balance(&self) -> Option<CipherText> {
    CipherText::decode(&mut self.enc_balance.as_slice()).ok()
  }
}

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default)]
#[cfg(feature = "backend")]
pub struct AccountAssetWithSecret {
  pub account_asset_id: i64,
  pub asset_id: i64,

  #[sqlx(flatten)]
  pub account: AccountWithSecret,

  pub balance: i64,
  pub enc_balance: Vec<u8>,
}

#[cfg(feature = "backend")]
impl AccountAssetWithSecret {
  pub fn enc_balance(&self) -> Option<CipherText> {
    CipherText::decode(&mut self.enc_balance.as_slice()).ok()
  }

  pub fn mint(&self, amount: Balance) -> Option<UpdateAccountAsset> {
    // Decode `enc_balance`.
    let enc_balance = self.enc_balance()?;
    // Update account balance.
    Some(UpdateAccountAsset {
      account_id: self.account.account_id,
      asset_id: self.asset_id,
      balance: (self.balance as u64) + amount,
      enc_balance: enc_balance + CipherText::value(amount.into()),
    })
  }

  pub fn create_send_tx(
    &self,
    req: &SenderProofRequest,
  ) -> Result<(UpdateAccountAsset, ConfidentialTransferProof), String> {
    // Decode MercatAccount from database.
    let sender = self
      .account
      .encryption_keys()
      .ok_or_else(|| format!("Failed to get account from database."))?;
    // Decode `req`.
    let enc_balance = req
      .encrypted_balance()?
      .or_else(|| self.enc_balance())
      .ok_or_else(|| format!("No encrypted balance."))?;
    let receiver = req.receiver()?;
    let auditor = req.auditor()?;

    let mut rng = rand::thread_rng();
    let sender_balance = self.balance as Balance;
    let tx = ConfidentialTransferProof::new(
      &sender,
      &enc_balance,
      sender_balance,
      &receiver,
      &BTreeMap::from([(AuditorId(0), auditor)]),
      req.amount,
      &mut rng,
    )
    .map_err(|e| format!("Failed to generate proof: {e:?}"))?;
    // Update account balance.
    let update = UpdateAccountAsset {
      account_id: self.account.account_id,
      asset_id: self.asset_id,
      balance: (self.balance as u64) - req.amount,
      enc_balance: enc_balance - tx.sender_amount(),
    };

    Ok((update, tx))
  }

  pub fn receiver_verify_tx(&self, req: &ReceiverVerifyRequest) -> Result<bool, String> {
    // Decode MercatAccount from database.
    let receiver = self
      .account
      .encryption_keys()
      .ok_or_else(|| format!("Failed to get account from database."))?;

    // Decode request.
    let sender_proof = req.sender_proof()?;
    sender_proof
      .receiver_verify(receiver, req.amount)
      .map_err(|e| format!("Failed to verify sender proof: {e:?}"))?;
    Ok(true)
  }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CreateAccountAsset {
  pub asset_id: i64,
}

#[derive(Clone, Debug, Default)]
#[cfg(feature = "backend")]
pub struct UpdateAccountAsset {
  pub account_id: i64,
  pub asset_id: i64,

  pub balance: Balance,
  pub enc_balance: CipherText,
}

#[cfg(feature = "backend")]
impl UpdateAccountAsset {
  pub fn enc_balance(&self) -> Vec<u8> {
    self.enc_balance.encode()
  }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AccountMintAsset {
  pub amount: Balance,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AccountAssetWithTx {
  pub account_asset: AccountAsset,
  #[serde(with = "SerHexSeq::<StrictPfx>")]
  pub tx: Vec<u8>,
}

#[cfg(feature = "backend")]
impl AccountAssetWithTx {
  pub fn new_send_tx(account_asset: AccountAsset, tx: ConfidentialTransferProof) -> Self {
    Self {
      account_asset,
      tx: tx.encode(),
    }
  }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SenderProofRequest {
  #[serde(default, with = "SerHexSeq::<StrictPfx>")]
  encrypted_balance: Vec<u8>,
  #[serde(with = "SerHexSeq::<StrictPfx>")]
  receiver: Vec<u8>,
  #[serde(default, with = "SerHexSeq::<StrictPfx>")]
  auditor: Vec<u8>,
  amount: Balance,
}

#[cfg(feature = "backend")]
impl SenderProofRequest {
  pub fn encrypted_balance(&self) -> Result<Option<CipherText>, String> {
    Ok(if self.encrypted_balance.is_empty() {
      None
    } else {
      Some(
        CipherText::decode(&mut self.encrypted_balance.as_slice())
          .map_err(|e| format!("Failed to decode 'encrypted_balance': {e:?}"))?,
      )
    })
  }

  pub fn receiver(&self) -> Result<ElgamalPublicKey, String> {
    ElgamalPublicKey::decode(&mut self.receiver.as_slice())
      .map_err(|e| format!("Failed to decode 'receiver': {e:?}"))
  }

  pub fn auditor(&self) -> Result<ElgamalPublicKey, String> {
    ElgamalPublicKey::decode(&mut self.auditor.as_slice())
      .map_err(|e| format!("Failed to decode 'auditor': {e:?}"))
  }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AuditorVerifyRequest {
  #[serde(with = "SerHexSeq::<StrictPfx>")]
  sender_proof: Vec<u8>,
  amount: Balance,
}

#[cfg(feature = "backend")]
impl AuditorVerifyRequest {
  pub fn sender_proof(&self) -> Result<ConfidentialTransferProof, String> {
    ConfidentialTransferProof::decode(&mut self.sender_proof.as_slice())
      .map_err(|e| format!("Failed to decode 'sender_proof': {e:?}"))
  }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ReceiverVerifyRequest {
  #[serde(with = "SerHexSeq::<StrictPfx>")]
  sender_proof: Vec<u8>,
  amount: Balance,
}

#[cfg(feature = "backend")]
impl ReceiverVerifyRequest {
  pub fn sender_proof(&self) -> Result<ConfidentialTransferProof, String> {
    ConfidentialTransferProof::decode(&mut self.sender_proof.as_slice())
      .map_err(|e| format!("Failed to decode 'sender_proof': {e:?}"))
  }
}
