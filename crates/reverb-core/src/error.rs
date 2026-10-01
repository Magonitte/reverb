use serde::{Serialize, Serializer};
use ts_rs::TS;

pub type CoreResult<T> = Result<T, CoreError>;

/// Erro do núcleo. Serializa como `{ kind, message, i18nKey? }` (arquitetura §15).
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Db(#[from] rusqlite::Error),
    #[error("{0}")]
    Internal(String),
    #[error("{message}")]
    Invalid {
        message: String,
        i18n_key: Option<String>,
    },
}

impl CoreError {
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid {
            message: message.into(),
            i18n_key: None,
        }
    }

    pub fn invalid_i18n(message: impl Into<String>, i18n_key: impl Into<String>) -> Self {
        Self::Invalid {
            message: message.into(),
            i18n_key: Some(i18n_key.into()),
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::Io(_) => "io",
            Self::Json(_) => "json",
            Self::Db(_) => "db",
            Self::Internal(_) => "internal",
            Self::Invalid { .. } => "invalid",
        }
    }

    pub fn i18n_key(&self) -> Option<&str> {
        match self {
            Self::Invalid { i18n_key, .. } => i18n_key.as_deref(),
            _ => None,
        }
    }
}

/// Formato serializado de `CoreError` (o que a UI recebe como erro de comando).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "CoreError", optional_fields)]
pub struct ErrorPayload {
    pub kind: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub i18n_key: Option<String>,
}

impl From<&CoreError> for ErrorPayload {
    fn from(err: &CoreError) -> Self {
        Self {
            kind: err.kind().to_string(),
            message: err.to_string(),
            i18n_key: err.i18n_key().map(str::to_string),
        }
    }
}

impl Serialize for CoreError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ErrorPayload::from(self).serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializa_kind_message_e_i18n_key() {
        let err = CoreError::invalid_i18n("pasta inválida", "errors.invalidFolder");
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["kind"], "invalid");
        assert_eq!(json["message"], "pasta inválida");
        assert_eq!(json["i18nKey"], "errors.invalidFolder");
    }

    #[test]
    fn omite_i18n_key_quando_ausente() {
        let json = serde_json::to_value(CoreError::invalid("x")).unwrap();
        assert_eq!(json["kind"], "invalid");
        assert!(json.get("i18nKey").is_none());
    }

    #[test]
    fn converte_erro_de_io() {
        let err: CoreError = std::io::Error::other("falhou").into();
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["kind"], "io");
        assert_eq!(json["message"], "falhou");
    }
}
