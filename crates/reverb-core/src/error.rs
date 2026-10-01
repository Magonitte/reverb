use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

pub type CoreResult<T> = Result<T, CoreError>;

/// Erro do núcleo. Serializa como `{ kind, message, i18nKey? }` (arquitetura §15).
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
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

impl Serialize for CoreError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let key = self.i18n_key();
        let mut state =
            serializer.serialize_struct("CoreError", if key.is_some() { 3 } else { 2 })?;
        state.serialize_field("kind", self.kind())?;
        state.serialize_field("message", &self.to_string())?;
        if let Some(key) = key {
            state.serialize_field("i18nKey", key)?;
        }
        state.end()
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
