use thiserror::Error;

#[cfg(feature = "backend")]
use actix_web::{
  error::ResponseError,
  http::{header::ContentType, StatusCode},
  HttpResponse,
};

#[derive(Error, Debug)]
pub enum Error {
  #[error("Confidential asset error: {0}")]
  #[cfg(feature = "backend")]
  ConfidentialAssetError(#[from] confidential_assets::Error),

  #[error("other error: {0}")]
  Other(String),

  #[error("Database error: {0}")]
  Database(#[from] sqlx::Error),

  #[error("Json error: {0}")]
  Json(#[from] serde_json::Error),

  #[error("hex error: {0}")]
  Hex(#[from] hex::FromHexError),

  #[error("parity-scale-codec error: {0}")]
  #[cfg(feature = "backend")]
  ParityScaleCodec(#[from] codec::Error),

  #[error("sp-core crypto secret error: {0}")]
  SecretStringError(String),

  #[error("sp-core crypto error: {0}")]
  CoreCryptoError(String),

  #[error("{0} not found")]
  NotFound(String),
}

impl Error {
  pub fn other(msg: &str) -> Self {
    Self::Other(msg.to_string())
  }

  pub fn not_found(msg: &str) -> Self {
    Self::NotFound(msg.to_string())
  }
}

impl From<sp_core::crypto::SecretStringError> for Error {
  fn from(e: sp_core::crypto::SecretStringError) -> Self {
    Self::SecretStringError(format!("{e:?}"))
  }
}

impl From<sp_core::crypto::PublicError> for Error {
  fn from(e: sp_core::crypto::PublicError) -> Self {
    Self::CoreCryptoError(format!("{e:?}"))
  }
}

#[cfg(feature = "backend")]
impl ResponseError for Error {
  fn error_response(&self) -> HttpResponse {
    HttpResponse::build(self.status_code())
      .insert_header(ContentType::html())
      .body(self.to_string())
  }

  fn status_code(&self) -> StatusCode {
    match self {
      Self::NotFound(_) => StatusCode::NOT_FOUND,
      _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
  }
}

pub type Result<T, E = Error> = core::result::Result<T, E>;
