use serde::Serialize;
use ts_rs::TS;

/// Informações básicas do app enviadas à UI (`app_info`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppInfo {
    pub version: String,
    pub platform: String,
    pub arch: String,
}

impl AppInfo {
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into(),
            platform: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializa_em_camel_case() {
        let json = serde_json::to_value(AppInfo::new("1.2.3")).unwrap();
        assert_eq!(json["version"], "1.2.3");
        assert!(json["platform"].is_string());
        assert!(json["arch"].is_string());
    }
}
