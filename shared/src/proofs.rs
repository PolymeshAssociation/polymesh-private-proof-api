use serde::{Deserialize, Serialize};
use serde_hex::{SerHexSeq, StrictPfx};

use utoipa::ToSchema;

use zeroize::{Zeroize, ZeroizeOnDrop};

#[cfg(feature = "backend")]
use codec::{Decode, Encode};

#[cfg(feature = "backend")]
use polymesh_api::types::pallet_confidential_asset::{ConfidentialAccount, MediatorAccount};

#[cfg(feature = "backend")]
use confidential_assets::{
  elgamal::CipherText,
  transaction::{AuditorId, ConfidentialTransferProof, MAX_TOTAL_SUPPLY},
  Balance, ElgamalKeys, ElgamalPublicKey, ElgamalSecretKey, Scalar,
};

use crate::error::*;

#[cfg(not(feature = "backend"))]
pub type Balance = u64;

/// User for account access control.
#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct User {
  /// User id.
  #[schema(example = 1)]
  pub user_id: i64,
  /// User name.
  #[schema(example = "TestUser")]
  pub username: String,

  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

/// Create a new user.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct CreateUser {
  /// User name.
  #[schema(example = "TestUser")]
  pub username: String,
}

/// Asset.
#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct Asset {
  /// Asset id.
  #[schema(example = 1)]
  pub asset_id: i64,
  /// Asset ticker.
  #[schema(example = "ACME1")]
  pub ticker: String,

  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

/// Create an asset.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct CreateAsset {
  /// Asset ticker.
  #[schema(example = "ACME1")]
  pub ticker: String,
}

/// Confidential account.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct Account {
  /// Account id.
  #[schema(example = 1)]
  pub account_id: i64,

  /// Account public key (Elgamal public key).
  #[schema(example = "0xdeadbeef00000000000000000000000000000000000000000000000000000000")]
  #[serde(with = "SerHexSeq::<StrictPfx>")]
  pub public_key: Vec<u8>,

  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

/// Account with secret key.  Not allowed to be serialized.
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
  pub fn encryption_keys(&self) -> Result<ElgamalKeys> {
    Ok(ElgamalKeys {
      public: ElgamalPublicKey::decode(&mut self.public_key.as_slice())?,
      secret: ElgamalSecretKey::decode(&mut self.secret_key.as_slice())?,
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

  pub fn auditor_verify_proof(&self, req: &AuditorVerifyRequest) -> Result<bool> {
    // Decode ConfidentialAccount from database.
    let auditor = self.encryption_keys()?;

    // Decode request.
    let sender_proof = req.sender_proof()?;

    let amount = sender_proof.auditor_verify(AuditorId(req.auditor_id), &auditor)?;
    if amount != req.amount {
      return Err(Error::other(
        "Failed to verify sender proof: Invalid transaction amount",
      ));
    }
    Ok(true)
  }
}

/// Create a new account.  Not allowed to be serialized.
#[derive(Clone, Debug, Default, Zeroize, ZeroizeOnDrop)]
pub struct CreateAccount {
  pub public_key: Vec<u8>,
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

/// Account asset.
#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct AccountAsset {
  /// Account asset id.
  #[schema(example = 1)]
  pub account_asset_id: i64,
  /// Account id.
  #[schema(example = 1)]
  pub account_id: i64,
  /// Asset id.
  #[schema(example = 1)]
  pub asset_id: i64,

  /// Current balance.
  #[schema(example = 1000)]
  pub balance: i64,
  /// Current balance encryted.
  #[schema(value_type = String, format = Binary, example = "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")]
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

/// Account asset with account secret key.  Not allowed to be serialized.
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
  pub fn enc_balance(&self) -> Result<CipherText> {
    Ok(CipherText::decode(&mut self.enc_balance.as_slice())?)
  }

  pub fn mint(&self, amount: Balance) -> Result<UpdateAccountAsset> {
    // Decode `enc_balance`.
    let enc_balance = self.enc_balance()?;
    // Update account balance.
    Ok(UpdateAccountAsset {
      account_id: self.account.account_id,
      asset_id: self.asset_id,
      balance: (self.balance as u64) + amount,
      enc_balance: enc_balance + CipherText::value(amount.into()),
    })
  }

  pub fn create_send_proof(
    &self,
    req: &SenderProofRequest,
  ) -> Result<(UpdateAccountAsset, ConfidentialTransferProof)> {
    // Decode ConfidentialAccount from database.
    let sender = self.account.encryption_keys()?;
    // Decode `req`.
    let enc_balance = req
      .encrypted_balance()?
      .or_else(|| self.enc_balance().ok())
      .ok_or_else(|| Error::other("No encrypted balance."))?;
    let receiver = req.receiver()?;
    let auditors = req
      .auditors()?
      .into_iter()
      .enumerate()
      .map(|(idx, auditor)| (AuditorId(idx as _), auditor))
      .collect();

    let mut rng = rand::thread_rng();
    let sender_balance = self.balance as Balance;
    let proof = ConfidentialTransferProof::new(
      &sender,
      &enc_balance,
      sender_balance,
      &receiver,
      &auditors,
      req.amount,
      &mut rng,
    )?;
    // Update account balance.
    let update = UpdateAccountAsset {
      account_id: self.account.account_id,
      asset_id: self.asset_id,
      balance: (self.balance as u64) - req.amount,
      enc_balance: enc_balance - proof.sender_amount(),
    };

    Ok((update, proof))
  }

  pub fn receiver_verify_proof(&self, req: &ReceiverVerifyRequest) -> Result<bool> {
    // Decode ConfidentialAccount from database.
    let receiver = self.account.encryption_keys()?;

    // Decode request.
    let sender_proof = req.sender_proof()?;
    sender_proof.receiver_verify(receiver, req.amount)?;
    Ok(true)
  }

  pub fn update_balance(
    &self,
    req: &UpdateAccountAssetBalanceRequest,
  ) -> Result<UpdateAccountAsset> {
    // Decode `req`.
    let enc_balance = req.encrypted_balance()?;
    // Decode ConfidentialAccount from database.
    let keys = self.account.encryption_keys()?;
    // Decrypt balance.
    let balance = keys
      .secret
      .decrypt_with_hint(&enc_balance, 0, MAX_TOTAL_SUPPLY)
      .ok_or_else(|| Error::other("Failed to decrypt balance."))?;
    // Update account balance.
    Ok(UpdateAccountAsset {
      account_id: self.account.account_id,
      asset_id: self.asset_id,
      balance,
      enc_balance,
    })
  }
}

/// Create a new account asset.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct CreateAccountAsset {
  /// Asset id.
  #[schema(example = 1)]
  pub asset_id: i64,
}

/// Update account asset.
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

/// Update account asset balance request.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateAccountAssetBalanceRequest {
  /// Encrypted balance.
  #[schema(value_type = String, format = Binary, example = "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")]
  #[serde(default, with = "SerHexSeq::<StrictPfx>")]
  encrypted_balance: Vec<u8>,
}

#[cfg(feature = "backend")]
impl UpdateAccountAssetBalanceRequest {
  pub fn encrypted_balance(&self) -> Result<CipherText> {
    Ok(CipherText::decode(&mut self.encrypted_balance.as_slice())?)
  }
}

/// Mint assets.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct AccountMintAsset {
  /// Amount to mint.
  #[schema(example = 1000, value_type = u64)]
  pub amount: Balance,
}

/// Account asset with sender proof.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct AccountAssetWithProof {
  /// Account asset.
  pub account_asset: AccountAsset,
  /// Sender proof.
  #[serde(with = "SerHexSeq::<StrictPfx>")]
  pub proof: Vec<u8>,
}

#[cfg(feature = "backend")]
impl AccountAssetWithProof {
  pub fn new_send_proof(account_asset: AccountAsset, proof: ConfidentialTransferProof) -> Self {
    Self {
      account_asset,
      proof: proof.encode(),
    }
  }
}

/// Elgamal public key.
#[derive(
  Clone, Debug, Default, Deserialize, Serialize, ToSchema, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct PublicKey(
  #[schema(example = "0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114")]
  #[serde(with = "SerHexSeq::<StrictPfx>")]
  Vec<u8>,
);

#[cfg(feature = "backend")]
impl PublicKey {
  pub fn decode(&self) -> Result<ElgamalPublicKey> {
    Ok(ElgamalPublicKey::decode(&mut self.0.as_slice())?)
  }

  pub fn as_confidential_account(&self) -> Result<ConfidentialAccount> {
    Ok(ConfidentialAccount::decode(&mut self.0.as_slice())?)
  }

  pub fn as_mediator_account(&self) -> Result<MediatorAccount> {
    Ok(MediatorAccount::decode(&mut self.0.as_slice())?)
  }
}

/// Confidential transfer sender proof.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct SenderProof(
  #[schema(example = "<Hex encoded sender proof>")]
  #[serde(with = "SerHexSeq::<StrictPfx>")]
  Vec<u8>,
);

#[cfg(feature = "backend")]
impl SenderProof {
  pub fn decode(&self) -> Result<ConfidentialTransferProof> {
    Ok(ConfidentialTransferProof::decode(&mut self.0.as_slice())?)
  }
}

/// Generate a new sender proof.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct SenderProofRequest {
  /// Current encrypted balance (optional).
  #[schema(value_type = String, format = Binary, example = "")]
  #[serde(default, with = "SerHexSeq::<StrictPfx>")]
  encrypted_balance: Vec<u8>,
  /// Receiver's public key.
  #[schema(value_type = String, format = Binary, example = "0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114")]
  receiver: PublicKey,
  /// List of auditors/mediators.  The order must match on-chain leg.
  #[schema(example = json!(["0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114"]))]
  #[serde(default)]
  auditors: Vec<PublicKey>,
  /// Transaction amount.
  #[schema(example = 1000, value_type = u64)]
  amount: Balance,
}

#[cfg(feature = "backend")]
impl SenderProofRequest {
  pub fn encrypted_balance(&self) -> Result<Option<CipherText>> {
    Ok(if self.encrypted_balance.is_empty() {
      None
    } else {
      Some(CipherText::decode(&mut self.encrypted_balance.as_slice())?)
    })
  }

  pub fn receiver(&self) -> Result<ElgamalPublicKey> {
    Ok(self.receiver.decode()?)
  }

  pub fn auditors(&self) -> Result<Vec<ElgamalPublicKey>> {
    Ok(
      self
        .auditors
        .iter()
        .map(|k| k.decode())
        .collect::<Result<Vec<_>>>()?,
    )
  }
}

/// SenderProof verify sender proof.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct SenderProofVerifyRequest {
  /// Sender's encrypted balance.
  #[schema(value_type = String, format = Binary, example = "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")]
  #[serde(default, with = "SerHexSeq::<StrictPfx>")]
  sender_balance: Vec<u8>,
  /// Sender's public key.
  #[schema(value_type = String, format = Binary, example = "0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114")]
  sender: PublicKey,
  /// Receiver's public key.
  #[schema(value_type = String, format = Binary, example = "0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114")]
  receiver: PublicKey,
  /// List of auditors/mediators.  The order must match on-chain leg.
  #[schema(example = json!(["0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114"]))]
  #[serde(default)]
  auditors: Vec<PublicKey>,
  /// Sender proof.
  sender_proof: SenderProof,
}

#[cfg(feature = "backend")]
impl SenderProofVerifyRequest {
  pub fn sender_balance(&self) -> Result<CipherText> {
    Ok(CipherText::decode(&mut self.sender_balance.as_slice())?)
  }

  pub fn sender(&self) -> Result<ElgamalPublicKey> {
    Ok(self.sender.decode()?)
  }

  pub fn receiver(&self) -> Result<ElgamalPublicKey> {
    Ok(self.receiver.decode()?)
  }

  pub fn auditors(&self) -> Result<Vec<ElgamalPublicKey>> {
    Ok(
      self
        .auditors
        .iter()
        .map(|k| k.decode())
        .collect::<Result<Vec<_>>>()?,
    )
  }

  pub fn sender_proof(&self) -> Result<ConfidentialTransferProof> {
    self.sender_proof.decode()
  }

  pub fn verify_proof(&self) -> Result<bool> {
    // Decode sender's balance.
    let sender_balance = self.sender_balance()?;
    // Decode sender & receiver.
    let sender = self.sender()?;
    let receiver = self.sender()?;
    let auditors = self
      .auditors()?
      .into_iter()
      .enumerate()
      .map(|(idx, auditor)| (AuditorId(idx as _), auditor))
      .collect();

    let mut rng = rand::thread_rng();
    let sender_proof = self.sender_proof()?;
    sender_proof.verify(&sender, &sender_balance, &receiver, &auditors, &mut rng)?;
    Ok(true)
  }
}

/// Verify result.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct SenderProofVerifyResult {
  /// Is the sender proof valid.
  #[schema(example = true)]
  is_valid: bool,
  /// If `is_valid` is false, then provide an error message.
  #[schema(example = json!(null))]
  err_msg: Option<String>,
}

#[cfg(feature = "backend")]
impl SenderProofVerifyResult {
  pub fn from_result(res: Result<bool>) -> Self {
    let (is_valid, err_msg) = match res {
      Ok(true) => (true, None),
      Ok(false) => (false, Some("Invalid proof".to_string())),
      Err(err) => (false, Some(format!("Invalid proof: {err:?}"))),
    };
    Self { is_valid, err_msg }
  }
}

/// Auditor verify sender proof.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditorVerifyRequest {
  /// Sender proof.
  sender_proof: SenderProof,
  /// Auditor id.
  #[schema(example = 0, value_type = u32)]
  auditor_id: u32,
  /// Transaction amount.
  #[schema(example = 1000, value_type = u64)]
  amount: Balance,
}

#[cfg(feature = "backend")]
impl AuditorVerifyRequest {
  pub fn sender_proof(&self) -> Result<ConfidentialTransferProof> {
    self.sender_proof.decode()
  }
}

/// Receiver verify sender proof.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct ReceiverVerifyRequest {
  /// Sender proof.
  sender_proof: SenderProof,
  /// Transaction amount.
  #[schema(example = 1000, value_type = u64)]
  amount: Balance,
}

#[cfg(feature = "backend")]
impl ReceiverVerifyRequest {
  pub fn sender_proof(&self) -> Result<ConfidentialTransferProof> {
    self.sender_proof.decode()
  }
}
