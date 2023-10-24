use std::collections::BTreeMap;
use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{de, Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};

use actix_web::web::Data;

use reqwest::{header, Client, Method, Url};

use dashmap::DashMap;

use async_trait::async_trait;
use confidential_proof_shared::{error::*, CreateSigner, SignerInfo};

use polymesh_api::client::{AccountId, Error as ClientError, Signer};
use sp_core::ed25519::Signature;
use sp_runtime::MultiSignature;

use super::{AppSigningManager, SigningManagerTrait, TxSigner};

#[derive(Debug, Deserialize)]
struct VaultResponse<T> {
  #[serde(default)]
  data: Option<T>,
  #[serde(default)]
  errors: Option<Vec<String>>,
}

impl<T> VaultResponse<T>
where
  T: std::fmt::Debug + std::default::Default + de::DeserializeOwned,
{
  async fn from_response(resp: reqwest::Response) -> Result<Option<T>> {
    let res: Self = resp.json().await?;
    match res {
      Self {
        errors: Some(errors),
        ..
      } => Err(Error::Other(format!("Vault error: {errors:?}"))),
      Self { errors: None, data } => Ok(data),
    }
  }
}

#[derive(Default, Debug, Deserialize)]
struct ListKeys {
  keys: Vec<String>,
}

#[derive(Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyType {
  #[default]
  Aes128Gcm96,
  Aes256Gcm96,
  Chacha20Poly1305,
  Ed25519,
  EcdsaP256,
  EcdsaP384,
  EcdsaP521,
  Rsa2048,
  Rsa3072,
  Rsa4096,
}

#[serde_as]
#[derive(Default, Debug, Deserialize)]
pub struct VersionedKey {
  #[serde_as(as = "Base64")]
  pub public_key: [u8; 32],
  pub creation_time: chrono::DateTime<chrono::Utc>,
}

impl VersionedKey {
  pub fn as_signer(&self, name: &str, version: u64) -> Result<SignerInfo> {
    Ok(SignerInfo {
      name: format!("{name}-{version}"),
      public_key: self.account().to_string(),
      created_at: self.creation_time.naive_utc(),
    })
  }

  pub fn account(&self) -> AccountId {
    AccountId::from(self.public_key)
  }
}

#[derive(Default, Debug, Deserialize)]
pub struct ReadKey {
  #[serde(rename = "type")]
  pub key_type: KeyType,
  pub deletion_allowed: bool,
  pub derived: bool,
  pub exportable: bool,
  pub allow_plaintext_backup: bool,
  pub keys: BTreeMap<u64, VersionedKey>,
  pub min_decryption_version: u64,
  pub min_encryption_version: u64,
  pub name: String,
  pub supports_encryption: bool,
  pub supports_decryption: bool,
  pub supports_derivation: bool,
  pub supports_signing: bool,
  pub imported: Option<bool>,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct CreateKeyRequest {
  #[serde(rename = "type")]
  pub key_type: KeyType,
  pub key_size: Option<u64>,
}

#[serde_as]
#[derive(Default, Debug, Deserialize, Serialize)]
pub struct SignRequest {
  pub key_version: u64,
  #[serde_as(as = "Base64")]
  pub input: Vec<u8>,
}

#[serde_as]
#[derive(Default, Debug, Deserialize)]
pub struct SignResponse {
  pub signature: String,
}

impl SignResponse {
  pub fn into_signature(self) -> Result<MultiSignature> {
    let sig = self
      .signature
      .strip_prefix("vault:v1:")
      .and_then(|encoded| STANDARD.decode(encoded).ok())
      .and_then(|data| Signature::from_slice(data.as_slice()));

    match sig {
      Some(sig) => Ok(sig.into()),
      None => Err(Error::other("Invalid signature from vault.")),
    }
  }
}

pub struct VaultSigner {
  pub client: Client,
  pub url: Url,
  pub key_version: u64,
  pub account: AccountId,
}

impl VaultSigner {
  async fn sign_data(&self, msg: &[u8]) -> Result<MultiSignature> {
    let req = SignRequest {
      key_version: self.key_version,
      input: msg.into(),
    };
    let resp = self.client.post(self.url.clone()).json(&req).send().await?;
    let signed = VaultResponse::<SignResponse>::from_response(resp)
      .await?
      .ok_or_else(|| Error::other("No signature from vault"))?;
    Ok(signed.into_signature()?)
  }
}

#[async_trait]
impl Signer for VaultSigner {
  fn account(&self) -> AccountId {
    self.account.clone()
  }

  async fn nonce(&self) -> Option<u32> {
    None
  }

  async fn set_nonce(&mut self, _nonce: u32) {}

  async fn sign(&self, msg: &[u8]) -> Result<MultiSignature, ClientError> {
    Ok(
      self
        .sign_data(msg)
        .await
        .map_err(|e| ClientError::SigningTransactionFailed(format!("{e:?}")))?,
    )
  }
}

pub struct VaultSigningManager {
  client: Client,
  list_url: Url,
  list: Method,
  keys_base: Url,
  sign_base: Url,
  keys: DashMap<String, SignerInfo>,
  //cache: DashMap<AccountId, VaultSigner>,
}

impl VaultSigningManager {
  pub fn new(base: String, token: String) -> Result<Arc<dyn SigningManagerTrait>> {
    let base = Url::parse(&base)?;
    let mut headers = header::HeaderMap::new();
    headers.insert("X-Vault-Token", header::HeaderValue::from_str(&token)?);
    let client = Client::builder().default_headers(headers).build()?;
    Ok(Arc::new(Self {
      client,
      list_url: base.join("./keys")?,
      list: Method::from_bytes(b"LIST")?,
      keys_base: base.join("./keys/")?,
      sign_base: base.join("./sign/")?,
      keys: DashMap::new(),
      //cache: DashMap::new(),
    }))
  }

  pub fn new_app_data(base: String, token: String) -> Result<AppSigningManager> {
    Ok(Data::from(Self::new(base, token)?))
  }

  pub fn get_key_url(&self, key: &str) -> Result<Url> {
    Ok(self.keys_base.join(key)?)
  }

  pub fn get_sign_url(&self, key: &str) -> Result<Url> {
    Ok(self.sign_base.join(key)?)
  }

  async fn vault_request<T>(&self, method: Method, url: Url) -> Result<Option<T>>
  where
    T: std::fmt::Debug + std::default::Default + de::DeserializeOwned,
  {
    let resp = self.client.request(method, url).send().await?;
    Ok(VaultResponse::from_response(resp).await?)
  }

  pub async fn fetch_keys(&self) -> Result<Vec<String>> {
    let data = self
      .vault_request::<ListKeys>(self.list.clone(), self.list_url.clone())
      .await?;
    Ok(data.unwrap_or_default().keys)
  }

  pub async fn fetch_key(&self, key: &str) -> Result<Option<ReadKey>> {
    let url = self.get_key_url(key)?;
    Ok(self.vault_request(Method::GET, url).await?)
  }

  pub async fn create_key(&self, key: &str) -> Result<Option<ReadKey>> {
    let req = CreateKeyRequest {
      key_type: KeyType::Ed25519,
      ..Default::default()
    };
    let url = self.get_key_url(key)?;
    let resp = self.client.post(url).json(&req).send().await?;
    Ok(VaultResponse::<ReadKey>::from_response(resp).await?)
  }
}

#[async_trait]
impl SigningManagerTrait for VaultSigningManager {
  async fn get_signers(&self) -> Result<Vec<SignerInfo>> {
    let mut signers = vec![];
    let keys = self.fetch_keys().await?;
    for key in keys {
      match self.fetch_key(&key).await? {
        Some(details) => {
          for (version, key) in details.keys {
            let signer = key.as_signer(&details.name, version)?;
            self.keys.insert(signer.name.clone(), signer.clone());
            signers.push(signer);
          }
        }
        None => (),
      }
    }
    Ok(signers)
  }

  async fn get_signer_info(&self, name: &str) -> Result<Option<SignerInfo>> {
    // Try to split `{name}-{version}`.
    let (name, version) = name
      .rsplit_once('-')
      .and_then(|(name, v)| v.parse().ok().map(|v| (name, v)))
      .unwrap_or_else(|| (name, 1));
    match self.fetch_key(name).await? {
      Some(details) => {
        for (key_version, key) in details.keys {
          if key_version != version {
            continue;
          }
          let signer = key.as_signer(&details.name, key_version)?;
          return Ok(Some(signer));
        }
      }
      None => (),
    }
    Ok(None)
  }

  async fn get_signer(&self, name: &str) -> Result<Option<TxSigner>> {
    // Try to split `{name}-{version}`.
    let (name, version) = name
      .rsplit_once('-')
      .and_then(|(name, v)| v.parse().ok().map(|v| (name, v)))
      .unwrap_or_else(|| (name, 1));
    match self.fetch_key(name).await? {
      Some(details) => {
        for (key_version, key) in details.keys {
          if key_version != version {
            continue;
          }
          let signer = VaultSigner {
            client: self.client.clone(),
            url: self.get_sign_url(&details.name)?,
            key_version,
            account: key.account(),
          };
          return Ok(Some(Box::new(signer)));
        }
      }
      None => (),
    }
    Ok(None)
  }

  async fn create_signer(&self, signer: &CreateSigner) -> Result<SignerInfo> {
    if signer.secret_uri.is_some() {
      return Err(Error::other(
        "VAULT signing manager doesn't support `secret_uri`.",
      ));
    }
    match self.create_key(&signer.name).await? {
      Some(details) if details.keys.len() > 0 => {
        let key = details
          .keys
          .get(&1)
          .ok_or_else(|| Error::other("No key returned"))?;
        Ok(key.as_signer(&details.name, 1)?)
      }
      _ => Err(Error::other("Failed to create key")),
    }
  }
}
