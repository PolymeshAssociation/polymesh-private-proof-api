use serde::{Deserialize, Serialize};
use serde_hex::{SerHexSeq, StrictPfx};

use utoipa::ToSchema;

use zeroize::{Zeroize, ZeroizeOnDrop};

#[cfg(feature = "backend")]
use codec::{Decode, Encode};

#[cfg(feature = "backend")]
use polymesh_api::{
  types::{
    pallet_confidential_asset::{
      MediatorAccount, ConfidentialAccount,
    },
  },
};

#[cfg(feature = "backend")]
use confidential_assets::{
  elgamal::CipherText,
  transaction::{AuditorId, ConfidentialTransferProof, MAX_TOTAL_SUPPLY},
  Balance, ElgamalKeys, ElgamalPublicKey, ElgamalSecretKey, Scalar,
};

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

  pub fn auditor_verify_proof(&self, req: &AuditorVerifyRequest) -> Result<bool, String> {
    // Decode ConfidentialAccount from database.
    let auditor = self
      .encryption_keys()
      .ok_or_else(|| format!("Failed to get account from database."))?;

    // Decode request.
    let sender_proof = req.sender_proof()?;

    let amount = sender_proof
      .auditor_verify(AuditorId(req.auditor_id), &auditor)
      .map_err(|e| format!("Failed to verify sender proof: {e:?}"))?;
    if amount != req.amount {
      return Err(format!("Failed to verify sender proof: Invalid transaction amount").into());
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
  #[schema(example = "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")]
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

  pub fn create_send_proof(
    &self,
    req: &SenderProofRequest,
  ) -> Result<(UpdateAccountAsset, ConfidentialTransferProof), String> {
    // Decode ConfidentialAccount from database.
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
    let auditors = req.auditors()?.into_iter().enumerate().map(|(idx, auditor)| {
      (AuditorId(idx as _), auditor)
    }).collect();

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
    )
    .map_err(|e| format!("Failed to generate proof: {e:?}"))?;
    // Update account balance.
    let update = UpdateAccountAsset {
      account_id: self.account.account_id,
      asset_id: self.asset_id,
      balance: (self.balance as u64) - req.amount,
      enc_balance: enc_balance - proof.sender_amount(),
    };

    Ok((update, proof))
  }

  pub fn receiver_verify_proof(&self, req: &ReceiverVerifyRequest) -> Result<bool, String> {
    // Decode ConfidentialAccount from database.
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

  pub fn update_balance(&self, req: &UpdateAccountAssetBalanceRequest) -> Result<UpdateAccountAsset, String> {
    // Decode `req`.
    let enc_balance = req
      .encrypted_balance()?;
    // Decode ConfidentialAccount from database.
    let keys = self
      .account
      .encryption_keys()
      .ok_or_else(|| format!("Failed to get account from database."))?;
    // Decrypt balance.
    let balance = keys.secret.decrypt_with_hint(&enc_balance, 0, MAX_TOTAL_SUPPLY)
      .ok_or_else(|| format!("Failed to decrypt balance."))?;
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
  pub fn encrypted_balance(&self) -> Result<CipherText, String> {
    Ok(CipherText::decode(&mut self.encrypted_balance.as_slice())
      .map_err(|e| format!("Failed to decode 'encrypted_balance': {e:?}"))?,
    )
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
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema, PartialEq, Eq, PartialOrd, Ord)]
pub struct PublicKey(
  #[schema(example = "0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114")]
  #[serde(with = "SerHexSeq::<StrictPfx>")]
  Vec<u8>
);

#[cfg(feature = "backend")]
impl PublicKey {
  pub fn decode(&self) -> Result<ElgamalPublicKey, String> {
    ElgamalPublicKey::decode(&mut self.0.as_slice())
      .map_err(|e| format!("Failed to decode PublicKey: {e:?}"))
  }

  pub fn as_confidential_account(&self) -> Result<ConfidentialAccount, String> {
    ConfidentialAccount::decode(&mut self.0.as_slice())
      .map_err(|e| format!("Failed to decode PublicKey: {e:?}"))
  }

  pub fn as_mediator_account(&self) -> Result<MediatorAccount, String> {
    MediatorAccount::decode(&mut self.0.as_slice())
      .map_err(|e| format!("Failed to decode PublicKey: {e:?}"))
  }
}

/// Confidential transfer sender proof.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct SenderProof(
  #[schema(example = "<Hex encoded sender proof>")]
  #[serde(with = "SerHexSeq::<StrictPfx>")]
  Vec<u8>
);

#[cfg(feature = "backend")]
impl SenderProof {
  pub fn decode(&self) -> Result<ConfidentialTransferProof, String> {
    ConfidentialTransferProof::decode(&mut self.0.as_slice())
      .map_err(|e| format!("Failed to decode 'sender_proof': {e:?}"))
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
    self.receiver.decode()
      .map_err(|e| format!("Failed to decode 'receiver': {e:?}"))
  }

  pub fn auditors(&self) -> Result<Vec<ElgamalPublicKey>, String> {
    Ok(self.auditors.iter().map(|k| {
      k.decode()
        .map_err(|e| format!("Failed to decode 'auditor': {e:?}"))
    }).collect::<Result<Vec<_>, String>>()?)
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
  pub fn sender_proof(&self) -> Result<ConfidentialTransferProof, String> {
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
  pub fn sender_proof(&self) -> Result<ConfidentialTransferProof, String> {
    self.sender_proof.decode()
  }
}
