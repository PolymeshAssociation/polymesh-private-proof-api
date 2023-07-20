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
    EncryptedAmount, PubAccount,
    InitializedAssetTx, InitializedTransferTx, PubAccountTx,
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

    pub fn mediator_verify_tx(&self, req: &MediatorVerifyRequest) -> Result<bool, String> {
        use mercat::{transaction::CtxMediator, AmountSource, TransferTransactionMediator};

        // Decode MercatAccount from database.
        let mediator = self.encryption_keys()
            .ok_or_else(|| format!("Failed to get account from database."))?;

        // Decode request.
        let sender_proof = req.sender_proof()?;
        let sender = req.sender()?;
        let sender_enc_balance = req.sender_enc_balance()?;
        let receiver = req.receiver()?;
        let amount = match req.amount {
            Some(amount) => AmountSource::Amount(amount),
            None => AmountSource::Encrypted(&mediator),
        };

        let mut rng = rand::thread_rng();
        CtxMediator
            .justify_transaction(
                &sender_proof,
                amount,
                &sender,
                &sender_enc_balance,
                &receiver,
                &[],
                &mut rng,
            )
            .map_err(|e| format!("Failed to verify sender proof: {e:?}"))?;
        Ok(true)
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

    pub fn create_send_tx(&self, req: &SenderProofRequest) -> Result<(UpdateAccountAsset, InitializedTransferTx), String> {
        use mercat::{transaction::CtxSender, TransferTransactionSender};
        // Decode MercatAccount from database.
        let sender = self.account.account()
            .ok_or_else(|| format!("Failed to get account from database."))?;
        // Decode `req`.
        let enc_balance = req.encrypted_balance()?
            .or_else(|| self.enc_balance())
            .ok_or_else(|| format!("No encrypted balance."))?;
        let receiver = req.receiver()?;
        let mediator = req.mediator()?
            .map(|k| k.owner_enc_pub_key);

        let mut rng = rand::thread_rng();
        let sender_balance = self.balance as Balance;
        let tx = CtxSender
            .create_transaction(
                &sender,
                &enc_balance,
                sender_balance,
                &receiver,
                mediator.as_ref(),
                &[],
                req.amount,
                &mut rng,
            )
            .map_err(|e| format!("Failed to generate proof: {e:?}"))?;
        // Update account balance.
        let update = UpdateAccountAsset {
            account_id: self.account.account_id,
            asset_id: self.asset_id,
            balance: (self.balance as u64) - req.amount,
            enc_balance: enc_balance - tx.memo.enc_amount_using_sender,
        };

        Ok((update, tx))
    }

    pub fn receiver_verify_tx(&self, req: &ReceiverVerifyRequest) -> Result<bool, String> {
        use mercat::{transaction::CtxReceiver, TransferTransactionReceiver};
        // Decode MercatAccount from database.
        let receiver = self.account.account()
            .ok_or_else(|| format!("Failed to get account from database."))?;

        // Decode request.
        let sender_proof = req.sender_proof()?;
        CtxReceiver
            .finalize_transaction(&sender_proof, receiver, req.amount)
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
    pub enc_balance: EncryptedAmount,
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
    pub fn new_init_tx(account_asset: AccountAsset, tx: PubAccountTx) -> Self {
        Self {
            account_asset,
            tx: tx.encode(),
        }
    }
    pub fn new_mint_tx(account_asset: AccountAsset, tx: InitializedAssetTx) -> Self {
        Self {
            account_asset,
            tx: tx.encode(),
        }
    }

    pub fn new_send_tx(account_asset: AccountAsset, tx: InitializedTransferTx) -> Self {
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
    mediator: Vec<u8>,
    amount: Balance,
}

#[cfg(feature = "backend")]
impl SenderProofRequest {
    pub fn encrypted_balance(&self) -> Result<Option<EncryptedAmount>, String> {
        Ok(if self.encrypted_balance.is_empty() {
            None
        } else {
            Some(EncryptedAmount::decode(&mut self.encrypted_balance.as_slice())
                .map_err(|e| format!("Failed to decode 'encrypted_balance': {e:?}"))?)
        })
    }

    pub fn receiver(&self) -> Result<PubAccount, String> {
        PubAccount::decode(&mut self.receiver.as_slice())
            .map_err(|e| format!("Failed to decode 'receiver': {e:?}"))
    }

    pub fn mediator(&self) -> Result<Option<PubAccount>, String> {
        Ok(if self.mediator.is_empty() {
            None
        } else {
            Some(PubAccount::decode(&mut self.mediator.as_slice())
                .map_err(|e| format!("Failed to decode 'mediator': {e:?}"))?)
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MediatorVerifyRequest {
    #[serde(with = "SerHexSeq::<StrictPfx>")]
    sender_proof: Vec<u8>,
    #[serde(with = "SerHexSeq::<StrictPfx>")]
    sender: Vec<u8>,
    #[serde(default, with = "SerHexSeq::<StrictPfx>")]
    sender_enc_balance: Vec<u8>,
    #[serde(with = "SerHexSeq::<StrictPfx>")]
    receiver: Vec<u8>,
    #[serde(default)]
    amount: Option<Balance>,
}

#[cfg(feature = "backend")]
impl MediatorVerifyRequest {
    pub fn sender_proof(&self) -> Result<InitializedTransferTx, String> {
        InitializedTransferTx::decode(&mut self.sender_proof.as_slice())
            .map_err(|e| format!("Failed to decode 'sender_proof': {e:?}"))
    }

    pub fn sender(&self) -> Result<PubAccount, String> {
        PubAccount::decode(&mut self.sender.as_slice())
            .map_err(|e| format!("Failed to decode 'sender': {e:?}"))
    }

    pub fn sender_enc_balance(&self) -> Result<EncryptedAmount, String> {
        EncryptedAmount::decode(&mut self.sender_enc_balance.as_slice())
                .map_err(|e| format!("Failed to decode 'sender_enc_balance': {e:?}"))
    }

    pub fn receiver(&self) -> Result<PubAccount, String> {
        PubAccount::decode(&mut self.receiver.as_slice())
            .map_err(|e| format!("Failed to decode 'receiver': {e:?}"))
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
    pub fn sender_proof(&self) -> Result<InitializedTransferTx, String> {
        InitializedTransferTx::decode(&mut self.sender_proof.as_slice())
            .map_err(|e| format!("Failed to decode 'sender_proof': {e:?}"))
    }
}
