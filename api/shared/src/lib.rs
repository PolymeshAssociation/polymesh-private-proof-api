use serde::{Deserialize, Serialize};
use serde_hex::{SerHexSeq,StrictPfx};

use zeroize::{Zeroize, ZeroizeOnDrop};

#[cfg(feature = "backend")]
use codec::{Decode, Encode};

#[cfg(feature = "backend")]
use mercat::{
    confidential_identity_core::{
        asset_proofs::Balance,
        curve25519_dalek::scalar::Scalar,
    },
    Account as MercatAccount, EncryptionKeys, EncryptionSecKey, EncryptionPubKey, SecAccount,
    EncryptedAmount,
    InitializedAssetTx, PubAccountTx,
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
#[derive(Clone, Debug, Default)]
#[derive(Zeroize, ZeroizeOnDrop)]
#[cfg(feature = "backend")]
pub struct AccountWithSecret {
    pub account_id: i64,

    pub public_key: Vec<u8>,
    pub secret_key: Vec<u8>,
}

#[cfg(feature = "backend")]
impl AccountWithSecret {
    pub fn encryption_keys(&self) -> Option<EncryptionKeys> {
        Some(EncryptionKeys {
          public: EncryptionPubKey::decode(&mut self.public_key.as_slice()).ok()?,
          secret: EncryptionSecKey::decode(&mut self.secret_key.as_slice()).ok()?,
        })
    }

    pub fn sec_account(&self) -> Option<SecAccount> {
        self.encryption_keys().map(SecAccount::from)
    }

    pub fn account(&self) -> Option<MercatAccount> {
        self.encryption_keys().map(MercatAccount::from)
    }

    pub fn init_balance_tx(&self, asset_id: i64) -> Option<(UpdateAccountAsset, PubAccountTx)> {
        self.sec_account().and_then(|account| {
            use mercat::{account::AccountCreator, AccountCreatorInitializer};
            let mut rng = rand::thread_rng();
            AccountCreator.create(&account, &mut rng).map(|tx| {
                let update = UpdateAccountAsset {
                    account_id: self.account_id,
                    asset_id,
                    balance: 0,
                    enc_balance: tx.initial_balance,
                };
                (update, tx)
            }).ok()
        })
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
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
    pub fn enc_balance(&self) -> Option<EncryptedAmount> {
        EncryptedAmount::decode(&mut self.enc_balance.as_slice()).ok()
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
    pub fn enc_balance(&self) -> Option<EncryptedAmount> {
        EncryptedAmount::decode(&mut self.enc_balance.as_slice()).ok()
    }

    pub fn create_mint_tx(&self, amount: Balance) -> Option<(UpdateAccountAsset, InitializedAssetTx)> {
        use mercat::{asset::AssetIssuer, AssetTransactionIssuer};
        // Decode `enc_balance`.
        let enc_balance = self.enc_balance()?;
        // Decode MercatAccount from database.
        let account = self.account.account()?;
        // Generate Asset mint proof.
        let mut rng = rand::thread_rng();
        let mint_tx = AssetIssuer
            .initialize_asset_transaction(&account, &[], amount, &mut rng)
            .ok()?;
        // Update account balance.
        let update = UpdateAccountAsset {
            account_id: self.account.account_id,
            asset_id: self.asset_id,
            balance: (self.balance as u64) + amount,
            enc_balance: enc_balance + mint_tx.memo.enc_issued_amount,
        };

        Some((update, mint_tx))
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
    pub enc_balance: EncryptedAmount,
}

#[cfg(feature = "backend")]
impl UpdateAccountAsset {
    pub fn enc_balance(&self) -> Vec<u8> {
        self.enc_balance.encode()
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AccountAssetWithInitTx {
    pub account_asset: AccountAsset,
    #[serde(with = "SerHexSeq::<StrictPfx>")]
    pub init_tx: Vec<u8>,
}

#[cfg(feature = "backend")]
impl AccountAssetWithInitTx {
    pub fn new(account_asset: AccountAsset, init_tx: PubAccountTx) -> Self {
        Self {
            account_asset,
            init_tx: init_tx.encode(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AccountMintAsset {
    pub amount: Balance,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AccountAssetWithMintTx {
    pub account_asset: AccountAsset,
    #[serde(with = "SerHexSeq::<StrictPfx>")]
    pub mint_tx: Vec<u8>,
}

#[cfg(feature = "backend")]
impl AccountAssetWithMintTx {
    pub fn new(account_asset: AccountAsset, mint_tx: InitializedAssetTx) -> Self {
        Self {
            account_asset,
            mint_tx: mint_tx.encode(),
        }
    }
}
