use std::collections::BTreeSet;

use uuid::Uuid;

use serde::{Deserialize, Serialize};
use serde_hex::{SerHex, SerHexSeq, StrictPfx};

use utoipa::ToSchema;

use zeroize::{Zeroize, ZeroizeOnDrop};

#[cfg(feature = "backend")]
use codec::{Decode, Encode};

#[cfg(feature = "tx_backend")]
use polymesh_api::types::pallet_confidential_asset::{AuditorAccount, ConfidentialAccount};

#[cfg(feature = "backend")]
use confidential_assets::{
  burn::ConfidentialBurnProof,
  elgamal::CipherText,
  transaction::{ConfidentialTransferProof, MAX_TOTAL_SUPPLY},
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
  pub asset_id: Uuid,

  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

/// Add an asset to the database.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct AddAsset {
  /// Asset id.
  pub asset_id: Uuid,
}

/// Confidential account.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct Account {
  /// Account id.
  #[serde(skip)]
  pub account_id: i64,

  /// Confidential account (Elgamal public key).
  #[schema(example = "0xdeadbeef00000000000000000000000000000000000000000000000000000000")]
  #[serde(with = "SerHexSeq::<StrictPfx>")]
  pub confidential_account: Vec<u8>,

  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[cfg(feature = "tx_backend")]
impl Account {
  pub fn as_confidential_account(&self) -> Result<ConfidentialAccount> {
    Ok(ConfidentialAccount::decode(
      &mut self.confidential_account.as_slice(),
    )?)
  }

  pub fn as_auditor_account(&self) -> Result<AuditorAccount> {
    Ok(AuditorAccount::decode(
      &mut self.confidential_account.as_slice(),
    )?)
  }
}

/// Account with secret key.  Not allowed to be serialized.
#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Zeroize, ZeroizeOnDrop)]
#[cfg(feature = "backend")]
pub struct AccountWithSecret {
  pub account_id: i64,

  pub confidential_account: Vec<u8>,
  pub secret_key: Vec<u8>,
}

#[cfg(feature = "backend")]
impl AccountWithSecret {
  pub fn match_confidential_account(&self, confidential_account: &PublicKey) -> bool {
    self.confidential_account.as_slice() == &confidential_account.0[..]
  }

  #[cfg(feature = "tx_backend")]
  pub fn as_confidential_account(&self) -> Result<ConfidentialAccount> {
    Ok(ConfidentialAccount::decode(
      &mut self.confidential_account.as_slice(),
    )?)
  }

  pub fn encryption_keys(&self) -> Result<ElgamalKeys> {
    Ok(ElgamalKeys {
      public: ElgamalPublicKey::decode(&mut self.confidential_account.as_slice())?,
      secret: ElgamalSecretKey::decode(&mut self.secret_key.as_slice())?,
    })
  }

  pub fn decrypt(&self, enc_value: &CipherText) -> Result<Balance> {
    // Decode ConfidentialAccount from database.
    let keys = self.encryption_keys()?;
    // Decrypt value.
    let value = keys
      .secret
      .decrypt_with_hint(enc_value, 0, MAX_TOTAL_SUPPLY)
      .ok_or_else(|| Error::other("Failed to decrypt value."))?;
    Ok(value)
  }

  pub fn apply_incoming(
    &self,
    asset_id: Uuid,
    enc_incoming: CipherText,
  ) -> Result<UpdateAccountAsset> {
    // Decode ConfidentialAccount from database.
    let keys = self.encryption_keys()?;
    // Decrypt incoming balance.
    let incoming_balance = keys
      .secret
      .decrypt_with_hint(&enc_incoming, 0, MAX_TOTAL_SUPPLY)
      .ok_or_else(|| Error::other("Failed to decrypt incoming balance."))?;
    // Update account balance.
    Ok(UpdateAccountAsset {
      account_asset_id: None,
      account_id: self.account_id,
      asset_id,
      balance: incoming_balance,
      enc_balance: enc_incoming,
    })
  }

  pub fn init_balance(&self, asset_id: Uuid) -> UpdateAccountAsset {
    UpdateAccountAsset {
      account_asset_id: None,
      account_id: self.account_id,
      asset_id,
      balance: 0,
      enc_balance: CipherText::zero(),
    }
  }

  pub fn auditor_verify_proof(
    &self,
    req: &AuditorVerifyRequest,
  ) -> Result<SenderProofVerifyResult> {
    // Decode ConfidentialAccount from database.
    let auditor = self.encryption_keys()?;

    // Decode sender proof from request.
    let sender_proof = req.sender_proof()?;

    let res = sender_proof
      .auditor_verify(req.auditor_id as u8, &auditor, req.amount)
      .map(|b| Some(b));
    Ok(SenderProofVerifyResult::from_result(res))
  }

  pub fn create_send_proof(
    &self,
    enc_balance: CipherText,
    balance: Option<Balance>,
    receiver: ElgamalPublicKey,
    auditors: BTreeSet<ElgamalPublicKey>,
    amount: Balance,
  ) -> Result<ConfidentialTransferProof> {
    // Decode ConfidentialAccount from database.
    let sender = self.encryption_keys()?;

    // Decrypted balance.
    let balance = match balance {
      None => sender
        .secret
        .decrypt_with_hint(&enc_balance, 0, MAX_TOTAL_SUPPLY)
        .ok_or_else(|| Error::other("Failed to decrypt balance."))?,
      Some(balance) => balance,
    };

    let mut rng = rand::thread_rng();
    let proof = ConfidentialTransferProof::new(
      &sender,
      &enc_balance,
      balance,
      &receiver,
      &auditors,
      amount,
      &mut rng,
    )?;

    Ok(proof)
  }

  pub fn create_burn_proof(
    &self,
    enc_balance: CipherText,
    balance: Option<Balance>,
    amount: Balance,
  ) -> Result<ConfidentialBurnProof> {
    // Decode ConfidentialAccount from database.
    let issuer = self.encryption_keys()?;

    // Decrypted balance.
    let balance = match balance {
      None => issuer
        .secret
        .decrypt_with_hint(&enc_balance, 0, MAX_TOTAL_SUPPLY)
        .ok_or_else(|| Error::other("Failed to decrypt balance."))?,
      Some(balance) => balance,
    };

    let mut rng = rand::thread_rng();
    Ok(ConfidentialBurnProof::new(
      &issuer,
      &enc_balance,
      balance,
      amount,
      &mut rng,
    )?)
  }

  pub fn receiver_verify_proof(
    &self,
    req: &ReceiverVerifyRequest,
  ) -> Result<SenderProofVerifyResult> {
    // Decode ConfidentialAccount from database.
    let receiver = self.encryption_keys()?;

    // Decode sender proof from request.
    let sender_proof = req.sender_proof()?;

    let res = sender_proof
      .receiver_verify(receiver, req.amount)
      .map(|b| Some(b));
    Ok(SenderProofVerifyResult::from_result(res))
  }

  pub fn decrypt_request(&self, req: &AccountDecryptRequest) -> Result<DecryptedResponse> {
    // Decode `req`.
    let enc_value = req.encrypted_value()?;
    // Decode ConfidentialAccount from database.
    let keys = self.encryption_keys()?;
    // Decrypt value.
    let value = keys
      .secret
      .decrypt_with_hint(&enc_value, 0, MAX_TOTAL_SUPPLY)
      .ok_or_else(|| Error::other("Failed to decrypt value."))?;
    // Return the decrypted value.
    Ok(DecryptedResponse { value })
  }
}

/// Create a new account.  Not allowed to be serialized.
#[derive(Clone, Debug, Default, Zeroize, ZeroizeOnDrop)]
pub struct CreateAccount {
  pub confidential_account: Vec<u8>,
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
      confidential_account: enc_keys.public.encode(),
      secret_key: enc_keys.secret.encode(),
    }
  }
}

/// Account asset.
#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct AccountAsset {
  /// Account asset id.
  #[serde(skip)]
  pub account_asset_id: i64,
  /// Account id.
  #[serde(skip)]
  pub account_id: i64,
  /// Asset id.
  pub asset_id: Uuid,

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
  pub fn enc_balance(&self) -> Result<CipherText> {
    Ok(CipherText::decode(&mut self.enc_balance.as_slice())?)
  }

  pub fn mint(&self, amount: Balance) -> Result<UpdateAccountAsset> {
    // Decode `enc_balance`.
    let enc_balance = self.enc_balance()?;
    // Update account balance.
    Ok(UpdateAccountAsset {
      account_asset_id: Some(self.account_asset_id),
      account_id: self.account_id,
      asset_id: self.asset_id.clone(),
      balance: (self.balance as u64) + amount,
      enc_balance: enc_balance + CipherText::value(amount.into()),
    })
  }
}

/// Account asset with account secret key.  Not allowed to be serialized.
#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default)]
#[cfg(feature = "backend")]
pub struct AccountAssetWithSecret {
  pub account_asset_id: i64,
  pub asset_id: Uuid,

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

  fn account_balance(&self, enc_balance: Option<CipherText>) -> Result<(CipherText, Balance)> {
    Ok(match enc_balance {
      Some(enc_balance) => {
        let balance = self.decrypt(&enc_balance)?;
        (enc_balance, balance)
      }
      None => (self.enc_balance()?, self.balance as Balance),
    })
  }

  pub fn create_send_proof(
    &self,
    enc_balance: Option<CipherText>,
    receiver: ElgamalPublicKey,
    auditors: BTreeSet<ElgamalPublicKey>,
    amount: Balance,
  ) -> Result<(UpdateAccountAsset, ConfidentialTransferProof)> {
    // Get sender's balance.
    let (enc_balance, balance) = self.account_balance(enc_balance)?;
    let proof =
      self
        .account
        .create_send_proof(enc_balance, Some(balance), receiver, auditors, amount)?;

    // Update account balance.
    let update = UpdateAccountAsset {
      account_asset_id: Some(self.account_asset_id),
      account_id: self.account.account_id,
      asset_id: self.asset_id.clone(),
      balance: (balance as u64) - amount,
      enc_balance: enc_balance - proof.sender_amount(),
    };

    Ok((update, proof))
  }

  pub fn create_burn_proof(
    &self,
    enc_balance: Option<CipherText>,
    amount: Balance,
  ) -> Result<(UpdateAccountAsset, ConfidentialBurnProof)> {
    // Get issuer's balance.
    let (enc_balance, balance) = self.account_balance(enc_balance)?;

    let proof = self
      .account
      .create_burn_proof(enc_balance, Some(balance), amount)?;
    // Update account balance.
    let enc_amount = CipherText::value(amount.into());
    let update = UpdateAccountAsset {
      account_asset_id: Some(self.account_asset_id),
      account_id: self.account.account_id,
      asset_id: self.asset_id.clone(),
      balance: (balance as u64) - amount,
      enc_balance: enc_balance - enc_amount,
    };

    Ok((update, proof))
  }

  pub fn receiver_verify_proof(
    &self,
    req: &ReceiverVerifyRequest,
  ) -> Result<SenderProofVerifyResult> {
    self.account.receiver_verify_proof(req)
  }

  pub fn decrypt(&self, enc_value: &CipherText) -> Result<Balance> {
    self.account.decrypt(enc_value)
  }

  pub fn decrypt_request(&self, req: &AccountDecryptRequest) -> Result<DecryptedResponse> {
    // Decode `req`.
    let enc_value = req.encrypted_value()?;
    // Decode ConfidentialAccount from database.
    let keys = self.account.encryption_keys()?;
    // Decrypt value.
    let value = keys
      .secret
      .decrypt_with_hint(&enc_value, 0, MAX_TOTAL_SUPPLY)
      .ok_or_else(|| Error::other("Failed to decrypt value."))?;
    // Return the decrypted value.
    Ok(DecryptedResponse { value })
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
      account_asset_id: Some(self.account_asset_id),
      account_id: self.account.account_id,
      asset_id: self.asset_id.clone(),
      balance,
      enc_balance,
    })
  }

  pub fn apply_incoming(&self, enc_incoming: CipherText) -> Result<UpdateAccountAsset> {
    // Decode ConfidentialAccount from database.
    let keys = self.account.encryption_keys()?;
    // Decrypt incoming balance.
    let incoming_balance = keys
      .secret
      .decrypt_with_hint(&enc_incoming, 0, MAX_TOTAL_SUPPLY)
      .ok_or_else(|| Error::other("Failed to decrypt incoming balance."))?;
    // Decode `enc_balance` from local DB.
    let enc_balance = self.enc_balance()?;
    // Update account balance.
    Ok(UpdateAccountAsset {
      account_asset_id: Some(self.account_asset_id),
      account_id: self.account.account_id,
      asset_id: self.asset_id.clone(),
      balance: (self.balance as u64) + incoming_balance,
      enc_balance: enc_balance + enc_incoming,
    })
  }
}

/// Create a new account asset.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct CreateAccountAsset {
  /// Asset id.
  pub asset_id: Uuid,
}

/// Update account asset.
#[derive(Clone, Debug, Default)]
#[cfg(feature = "backend")]
pub struct UpdateAccountAsset {
  pub account_asset_id: Option<i64>,
  pub account_id: i64,
  pub asset_id: Uuid,

  pub balance: Balance,
  pub enc_balance: CipherText,
}

#[cfg(feature = "backend")]
impl UpdateAccountAsset {
  pub fn init_balance(account_id: i64, asset_id: Uuid, balance: Balance) -> Self {
    Self {
      account_asset_id: None,
      account_id,
      asset_id,
      balance,
      enc_balance: CipherText::value(balance.into()),
    }
  }

  pub fn enc_balance(&self) -> Vec<u8> {
    self.enc_balance.encode()
  }
}

/// Decrypt a `CipherText` value request.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct AccountDecryptRequest {
  /// Encrypted value.
  #[schema(value_type = String, format = Binary, example = "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")]
  #[serde(default, with = "SerHexSeq::<StrictPfx>")]
  encrypted_value: Vec<u8>,
}

#[cfg(feature = "backend")]
impl AccountDecryptRequest {
  pub fn encrypted_value(&self) -> Result<CipherText> {
    Ok(CipherText::decode(&mut self.encrypted_value.as_slice())?)
  }
}

/// Decrypted incoming balance.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct DecryptedIncomingBalance {
  /// Decrypted incoming balance.
  #[schema(example = 1000)]
  pub incoming_balance: Option<u64>,
}

/// Decrypted value response.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct DecryptedResponse {
  /// Decrypted value.
  #[schema(example = 1000)]
  pub value: u64,
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

/// Account asset with sender/burn proof.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct AccountAssetWithProof {
  /// Account asset.
  pub account_asset: AccountAsset,
  /// Sender/burn proof.
  #[serde(with = "SerHexSeq::<StrictPfx>")]
  pub proof: Vec<u8>,
}

#[cfg(feature = "backend")]
impl AccountAssetWithProof {
  pub fn new_send_proof(account_asset: AccountAsset, proof: ConfidentialTransferProof) -> Self {
    Self {
      account_asset,
      proof: proof.as_bytes(),
    }
  }

  pub fn new_burn_proof(account_asset: AccountAsset, proof: ConfidentialBurnProof) -> Self {
    Self {
      account_asset,
      proof: proof.as_bytes(),
    }
  }
}

/// Elgamal public key.
#[derive(
  Clone,
  Debug,
  Default,
  Encode,
  Decode,
  Deserialize,
  Serialize,
  ToSchema,
  PartialEq,
  Eq,
  PartialOrd,
  Ord,
)]
pub struct PublicKey(
  #[schema(example = "0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114")]
  #[serde(with = "SerHex::<StrictPfx>")]
  pub [u8; 32],
);

#[cfg(feature = "backend")]
impl PublicKey {
  pub fn from_str(s: &str) -> Result<Self> {
    let mut bytes = [0u8; 32];
    if s.starts_with("0x") {
      hex::decode_to_slice(&s[2..], &mut bytes as &mut [u8])?;
    } else {
      hex::decode_to_slice(s, &mut bytes as &mut [u8])?;
    }
    Ok(Self(bytes))
  }

  pub fn decode(&self) -> Result<ElgamalPublicKey> {
    Ok(ElgamalPublicKey::decode(&mut &self.0[..])?)
  }

  #[cfg(feature = "tx_backend")]
  pub fn as_confidential_account(&self) -> Result<ConfidentialAccount> {
    Ok(ConfidentialAccount::decode(&mut &self.0[..])?)
  }

  #[cfg(feature = "tx_backend")]
  pub fn as_auditor_account(&self) -> Result<AuditorAccount> {
    Ok(AuditorAccount::decode(&mut &self.0[..])?)
  }
}

/// Confidential transfer proofs.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct TransferProofs {
  pub proofs: Vec<(Uuid, SenderProof)>,
}

/// Confidential transfer sender proof.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SenderProof(
  #[schema(example = "<Hex encoded sender proof>")]
  #[serde(with = "SerHexSeq::<StrictPfx>")]
  pub Vec<u8>,
);

#[cfg(feature = "backend")]
impl SenderProof {
  pub fn new(proof: ConfidentialTransferProof) -> Self {
    Self(proof.as_bytes())
  }

  pub fn decode(&self) -> Result<ConfidentialTransferProof> {
    Ok(ConfidentialTransferProof::from_bytes(
      &mut self.0.as_slice(),
    )?)
  }
}

/// Generate a new sender proof.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct SenderProofRequest {
  /// Current encrypted balance.
  #[schema(value_type = String, format = Binary, example = "")]
  #[serde(default, with = "SerHexSeq::<StrictPfx>")]
  encrypted_balance: Vec<u8>,
  /// Receiver's confidential account.
  #[schema(value_type = String, format = Binary, example = "0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114")]
  receiver: PublicKey,
  /// List of auditors.
  #[schema(example = json!(["0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114"]))]
  #[serde(default)]
  auditors: Vec<PublicKey>,
  /// Transaction amount.
  #[schema(example = 1000, value_type = u64)]
  pub amount: Balance,
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

  pub fn auditors(&self) -> Result<BTreeSet<ElgamalPublicKey>> {
    let mut auditors = BTreeSet::new();
    for k in &self.auditors {
      auditors.insert(k.decode()?);
    }
    Ok(auditors)
  }
}

/// SenderProof verify sender proof.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct SenderProofVerifyRequest {
  /// Sender's encrypted balance.
  #[schema(value_type = String, format = Binary, example = "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")]
  #[serde(default, with = "SerHexSeq::<StrictPfx>")]
  sender_balance: Vec<u8>,
  /// Sender's confidential account.
  #[schema(value_type = String, format = Binary, example = "0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114")]
  sender: PublicKey,
  /// Receiver's confidential account.
  #[schema(value_type = String, format = Binary, example = "0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114")]
  receiver: PublicKey,
  /// List of auditors.
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

  pub fn auditors(&self) -> Result<BTreeSet<ElgamalPublicKey>> {
    let mut auditors = BTreeSet::new();
    for k in &self.auditors {
      auditors.insert(k.decode()?);
    }
    Ok(auditors)
  }

  pub fn sender_proof(&self) -> Result<ConfidentialTransferProof> {
    self.sender_proof.decode()
  }

  pub fn verify_proof(&self) -> Result<SenderProofVerifyResult> {
    // Decode sender's balance.
    let sender_balance = self.sender_balance()?;
    // Decode sender & receiver.
    let sender = self.sender()?;
    let receiver = self.receiver()?;
    let auditors = self.auditors()?.into_iter().collect();

    let mut rng = rand::thread_rng();
    let sender_proof = self.sender_proof()?;

    let res = sender_proof
      .verify(&sender, &sender_balance, &receiver, &auditors, &mut rng)
      .map(|_| None);
    Ok(SenderProofVerifyResult::from_result(res))
  }
}

/// Verify result.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct SenderProofVerifyResult {
  /// Is the sender proof valid.
  #[schema(example = true)]
  is_valid: bool,
  /// The decrypted transaction amount (Only available when the receiver/auditor verified).
  #[schema(example = 1000, value_type = u64)]
  amount: Option<Balance>,
  /// If `is_valid` is false, then provide an error message.
  #[schema(example = json!(null))]
  err_msg: Option<String>,
}

#[cfg(feature = "backend")]
impl SenderProofVerifyResult {
  pub fn from_result<E: core::fmt::Debug>(res: Result<Option<Balance>, E>) -> Self {
    match res {
      Ok(amount) => Self {
        is_valid: true,
        amount,
        err_msg: None,
      },
      Err(err) => Self {
        is_valid: false,
        amount: None,
        err_msg: Some(format!("Invalid proof: {err:?}")),
      },
    }
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
  #[schema(example = json!(null), value_type = u64)]
  amount: Option<Balance>,
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
  #[schema(example = json!(null), value_type = u64)]
  amount: Option<Balance>,
}

#[cfg(feature = "backend")]
impl ReceiverVerifyRequest {
  pub fn sender_proof(&self) -> Result<ConfidentialTransferProof> {
    self.sender_proof.decode()
  }
}

/// Confidential burn burn proof.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct BurnProof(
  #[schema(example = "<Hex encoded burn proof>")]
  #[serde(with = "SerHexSeq::<StrictPfx>")]
  pub Vec<u8>,
);

#[cfg(feature = "backend")]
impl BurnProof {
  pub fn new(proof: ConfidentialBurnProof) -> Self {
    Self(proof.as_bytes())
  }

  pub fn decode(&self) -> Result<ConfidentialBurnProof> {
    Ok(ConfidentialBurnProof::from_bytes(&self.0)?)
  }
}

/// Generate a new burn proof.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct BurnProofRequest {
  /// Current encrypted balance.
  #[schema(value_type = String, format = Binary, example = "")]
  #[serde(default, with = "SerHexSeq::<StrictPfx>")]
  encrypted_balance: Vec<u8>,
  /// Transaction amount.
  #[schema(example = 1000, value_type = u64)]
  pub amount: Balance,
}

#[cfg(feature = "backend")]
impl BurnProofRequest {
  pub fn encrypted_balance(&self) -> Result<Option<CipherText>> {
    Ok(if self.encrypted_balance.is_empty() {
      None
    } else {
      Some(CipherText::decode(&mut self.encrypted_balance.as_slice())?)
    })
  }
}
