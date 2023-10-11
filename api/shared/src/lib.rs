use serde::{Deserialize, Serialize};
use serde_hex::{SerHexSeq, StrictPfx};

use utoipa::ToSchema;

use zeroize::{Zeroize, ZeroizeOnDrop};

#[cfg(feature = "backend")]
use codec::{Decode, Encode};

#[cfg(feature = "backend")]
use confidential_assets::{
  elgamal::CipherText,
  transaction::{AuditorId, ConfidentialTransferProof},
  Balance, ElgamalKeys, ElgamalPublicKey, ElgamalSecretKey, Scalar,
};

#[cfg(not(feature = "backend"))]
pub type Balance = u64;

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct User {
  #[schema(example = 1)]
  pub user_id: i64,
  #[schema(example = "TestUser")]
  pub username: String,

  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct CreateUser {
  #[schema(example = "TestUser")]
  pub username: String,
}

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct Asset {
  #[schema(example = 1)]
  pub asset_id: i64,
  #[schema(example = "ACME1")]
  pub ticker: String,

  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct CreateAsset {
  #[schema(example = "ACME1")]
  pub ticker: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct Account {
  #[schema(example = 1)]
  pub account_id: i64,

  #[schema(example = "0xdeadbeef00000000000000000000000000000000000000000000000000000000")]
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
    // Decode ConfidentialAccount from database.
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

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema, Zeroize, ZeroizeOnDrop)]
pub struct CreateAccount {
  #[schema(example = "0xdeadbeef00000000000000000000000000000000000000000000000000000000")]
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
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct AccountAsset {
  #[schema(example = 1)]
  pub account_asset_id: i64,
  #[schema(example = 1)]
  pub account_id: i64,
  #[schema(example = 1)]
  pub asset_id: i64,

  #[schema(example = 1000)]
  pub balance: i64,
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
    let tx = ConfidentialTransferProof::new(
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
      enc_balance: enc_balance - tx.sender_amount(),
    };

    Ok((update, tx))
  }

  pub fn receiver_verify_tx(&self, req: &ReceiverVerifyRequest) -> Result<bool, String> {
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
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct CreateAccountAsset {
  #[schema(example = 1)]
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

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct AccountMintAsset {
  #[schema(example = 1000, value_type = u64)]
  pub amount: Balance,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
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

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
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
}

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

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct SenderProofRequest {
  #[schema(value_type = String, format = Binary, example = "")]
  #[serde(default, with = "SerHexSeq::<StrictPfx>")]
  encrypted_balance: Vec<u8>,
  #[schema(value_type = String, format = Binary, example = "0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114")]
  receiver: PublicKey,
  #[schema(example = json!(["0xceae8587b3e968b9669df8eb715f73bcf3f7a9cd3c61c515a4d80f2ca59c8114"]))]
  #[serde(default)]
  auditors: Vec<PublicKey>,
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

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditorVerifyRequest {
  sender_proof: SenderProof,
  #[schema(example = 1000, value_type = u64)]
  amount: Balance,
}

#[cfg(feature = "backend")]
impl AuditorVerifyRequest {
  pub fn sender_proof(&self) -> Result<ConfidentialTransferProof, String> {
    self.sender_proof.decode()
  }
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct ReceiverVerifyRequest {
  sender_proof: SenderProof,
  #[schema(example = 1000, value_type = u64)]
  amount: Balance,
}

#[cfg(feature = "backend")]
impl ReceiverVerifyRequest {
  pub fn sender_proof(&self) -> Result<ConfidentialTransferProof, String> {
    self.sender_proof.decode()
  }
}
