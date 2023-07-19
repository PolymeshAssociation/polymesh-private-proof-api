use serde::{Deserialize, Serialize};
use serde_hex::{SerHexSeq,StrictPfx};

use zeroize::{Zeroize, ZeroizeOnDrop};

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[cfg(feature = "backend")]
use codec::{Decode, Encode};

#[cfg(feature = "backend")]
use mercat::{
    confidential_identity_core::{
        asset_proofs::{Balance, CipherText},
        curve25519_dalek::scalar::Scalar,
    },
    Account as MercatAccount, EncryptionKeys, EncryptionSecKey, EncryptionPubKey, SecAccount,
};

#[cfg(not(feature = "backend"))]
pub type Balance = u64;

pub const TOKEN_SCALE: Decimal = dec!(1_000_000);
pub const MAXIMUM_DECRYPT_RANGE: Decimal = dec!(1_000_000.0);

pub fn to_balance(val: Decimal) -> Option<Balance> {
    (val * TOKEN_SCALE)
        .to_u64()
        .and_then(|val| val.try_into().ok())
}

pub fn from_balance(val: Balance) -> Decimal {
    Decimal::from(val) / TOKEN_SCALE
}

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct User {
    pub user_id: i64,
    pub username: String,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CreateUser {
    pub username: String,
}

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Asset {
    pub asset_id: i64,
    pub ticker: String,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CreateAsset {
    pub ticker: String,
}

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Account {
    pub account_id: i64,

    #[serde(with = "SerHexSeq::<StrictPfx>")]
    pub public_key: Vec<u8>,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct AccountWithSecret {
    pub account_id: i64,

    #[serde(with = "SerHexSeq::<StrictPfx>")]
    pub public_key: Vec<u8>,
    #[serde(skip)]
    pub secret_key: Vec<u8>,

    #[zeroize(skip)]
    pub created_at: chrono::NaiveDateTime,
    #[zeroize(skip)]
    pub updated_at: chrono::NaiveDateTime,
}

impl AccountWithSecret {
    #[cfg(feature = "backend")]
    pub fn encryption_keys(&self) -> Option<EncryptionKeys> {
        Some(EncryptionKeys {
          public: EncryptionPubKey::decode(&mut self.public_key.as_slice()).ok()?,
          secret: EncryptionSecKey::decode(&mut self.secret_key.as_slice()).ok()?,
        })
    }

    #[cfg(feature = "backend")]
    pub fn sec_account(&self) -> Option<SecAccount> {
        self.encryption_keys().map(SecAccount::from)
    }

    #[cfg(feature = "backend")]
    pub fn account(&self) -> Option<MercatAccount> {
        self.encryption_keys().map(MercatAccount::from)
    }
}

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct CreateAccount {
    #[serde(with = "SerHexSeq::<StrictPfx>")]
    pub public_key: Vec<u8>,
    #[serde(skip)]
    pub secret_key: Vec<u8>,
}

#[cfg(feature = "backend")]
impl CreateAccount {
    fn create_secret_account() -> EncryptionKeys {
        let mut rng = rand::thread_rng();
        let secret = EncryptionSecKey::new(Scalar::random(&mut rng));
        let public = secret.get_public_key();
        EncryptionKeys {
            public,
            secret,
        }
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
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct AccountBalance {
    pub account_balance_id: i64,
    pub account_id: i64,
    pub asset_id: i64,

    pub balance: i64,
    #[serde(with = "SerHexSeq::<StrictPfx>")]
    pub enc_balance: Vec<u8>,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,

    // From `accounts`.
    //#[serde(skip)]
    //pub enc_keys: Vec<u8>,
}

impl AccountBalance {
    #[cfg(feature = "backend")]
    pub fn enc_balance(&self) -> Option<CipherText> {
        CipherText::decode(&mut self.enc_balance.as_slice()).ok()
    }

    pub fn balance(&self) -> Decimal {
        from_balance(self.balance as Balance)
    }
}

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CreateAccountBalance {
    pub account_id: i64,
    pub asset_id: i64,

    pub balance: Balance,
    pub enc_balance: Vec<u8>,
}
