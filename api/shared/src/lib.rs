use serde::{Deserialize, Serialize};

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[cfg(feature = "backend")]
use codec::{Decode, Encode};

#[cfg(feature = "backend")]
use mercat::{
    confidential_identity_core::{
        asset_proofs::{Balance, CipherText, ElgamalSecretKey},
        curve25519_dalek::scalar::Scalar,
    },
    Account as MercatAccount, EncryptionKeys, SecAccount,
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

#[cfg(feature = "backend")]
pub fn hex_encode<T: Encode>(val: T) -> String {
    format!("0x{}", hex::encode(val.encode()))
}

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct User {
    pub id: i64,
    pub name: String,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CreateUser {
    pub name: String,
}

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Asset {
    pub id: i64,
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
    pub id: i64,
    pub public_key: String,

    #[serde(skip)]
    pub enc_keys: Vec<u8>,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl Account {
    #[cfg(feature = "backend")]
    pub fn encryption_keys(&self) -> Option<EncryptionKeys> {
        EncryptionKeys::decode(&mut self.enc_keys.as_slice()).ok()
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
pub struct CreateAccount {
    pub public_key: String,

    #[serde(skip)]
    pub enc_keys: Vec<u8>,
}

#[cfg(feature = "backend")]
impl CreateAccount {
    fn create_secret_account() -> EncryptionKeys {
        let mut rng = rand::thread_rng();
        let elg_secret = ElgamalSecretKey::new(Scalar::random(&mut rng));
        let elg_pub = elg_secret.get_public_key();
        EncryptionKeys {
            public: elg_pub,
            secret: elg_secret,
        }
    }

    pub fn new() -> Self {
        let enc_keys = Self::create_secret_account();
        let public_key = hex_encode(&enc_keys.public);

        Self {
          public_key,
          enc_keys: enc_keys.encode(),
        }
    }
}

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct AccountBalance {
    pub id: i64,
    pub account_id: i64,
    pub asset_id: i64,

    pub balance: i64,
    pub enc_balance: Vec<u8>,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl AccountBalance {
    #[cfg(feature = "backend")]
    pub fn enc_balance(&self) -> Option<CipherText> {
        CipherText::decode(&mut self.enc_balance.as_slice()).ok()
    }

    pub fn enc_balance_hex(&self) -> String {
        format!("0x{}", hex::encode(self.enc_balance.as_slice()))
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
