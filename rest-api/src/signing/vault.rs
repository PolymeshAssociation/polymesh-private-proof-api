use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{de, Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};

use actix_web::web::Data;

use reqwest::{header, Client, Method, Url};

use dashmap::DashMap;

use async_trait::async_trait;
use polymesh_private_proof_shared::{error::*, CreateSigner, SignerInfo};

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
  pub fn as_signer(&self, name_version: &NameVersion) -> Result<SignerInfo> {
    Ok(SignerInfo {
      name: name_version.to_string(),
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

#[derive(Clone, Default, Debug, Hash, PartialEq, Eq)]
pub struct NameVersion {
  pub name: String,
  pub version: u64,
}

impl NameVersion {
  pub fn new(name: String, version: u64) -> Self {
    Self { name, version }
  }
}

impl FromStr for NameVersion {
  type Err = ();

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let (name, version) = s
      .rsplit_once('-')
      .and_then(|(name, v)| v.parse().ok().map(|v| (name, v)))
      .unwrap_or_else(|| (s, 1));
    Ok(Self {
      name: name.to_string(),
      version,
    })
  }
}

impl ToString for NameVersion {
  fn to_string(&self) -> String {
    format!("{}-{}", self.name, self.version)
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
  keys: DashMap<NameVersion, SignerInfo>,
  cache: DashMap<AccountId, NameVersion>,
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
      cache: DashMap::new(),
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

  fn info_to_vault_signer(&self, info: SignerInfo) -> Result<VaultSigner> {
    let name_version: NameVersion = info.name.parse().expect("Doesn't fail");
    Ok(VaultSigner {
      client: self.client.clone(),
      url: self.get_sign_url(&name_version.name)?,
      key_version: name_version.version,
      account: info.account_id()?,
    })
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

  fn cache_vault_key(
    &self,
    name: &str,
    key: VersionedKey,
    version: u64,
  ) -> Result<(AccountId, SignerInfo)> {
    let name_version = NameVersion::new(name.to_string(), version);
    let account = key.account();
    let signer = key.as_signer(&name_version)?;
    self.keys.insert(name_version.clone(), signer.clone());
    self.cache.insert(account, name_version);
    Ok((account, signer))
  }

  async fn load_vault_keys(
    &self,
    mut signers: Option<&mut Vec<SignerInfo>>,
    find: Option<AccountId>,
  ) -> Result<Option<SignerInfo>> {
    let keys = self.fetch_keys().await?;
    for key in keys {
      match self.fetch_key(&key).await? {
        Some(details) => {
          for (version, key) in details.keys {
            let (account, signer) = self.cache_vault_key(&details.name, key, version)?;
            if Some(account) == find {
              return Ok(Some(signer));
            }
            if let Some(signers) = &mut signers {
              signers.push(signer);
            }
          }
        }
        None => (),
      }
    }
    Ok(None)
  }

  async fn find_signer_info(&self, name: &str) -> Result<Option<SignerInfo>> {
    let name_version = match AccountId::from_str(name).ok() {
      // Search by account_id.
      Some(account_id) => {
        if let Some(name_version) = self.cache.get(&account_id) {
          // Looks like the account_id was loaded before.
          name_version.clone()
        } else {
          // Can't find the account_id.
          // Load vault keys and search for account_id.
          return self.load_vault_keys(None, Some(account_id)).await;
        }
      }
      None => {
        // Parse `{name}-{version}`.
        name.parse().expect("Doesn't fail")
      }
    };
    // Search by signer name/version.
    let signer = self.keys.get(&name_version).as_deref().cloned();
    if signer.is_some() {
      return Ok(signer);
    }

    // Load key from vault.
    match self.fetch_key(&name_version.name).await? {
      Some(details) => {
        for (version, key) in details.keys {
          let (_, signer) = self.cache_vault_key(&details.name, key, version)?;
          if version != name_version.version {
            continue;
          }
          return Ok(Some(signer));
        }
      }
      None => (),
    }
    Ok(None)
  }
}

#[async_trait]
impl SigningManagerTrait for VaultSigningManager {
  async fn get_signers(&self) -> Result<Vec<SignerInfo>> {
    let mut signers = vec![];
    self.load_vault_keys(Some(&mut signers), None).await?;
    Ok(signers)
  }

  async fn get_signer_info(&self, name: &str) -> Result<Option<SignerInfo>> {
    self.find_signer_info(name).await
  }

  async fn get_signer(&self, name: &str) -> Result<Option<TxSigner>> {
    let info = self.get_signer_info(name).await?;
    Ok(match info {
      Some(info) => Some(Box::new(self.info_to_vault_signer(info)?)),
      _ => None,
    })
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
        let name_version = NameVersion::new(details.name, 1);
        Ok(key.as_signer(&name_version)?)
      }
      _ => Err(Error::other("Failed to create key")),
    }
  }
}
