//! Configurações (arquitetura §6). O backend é a fonte da verdade; a UI recebe `SettingsView`
//! (sem segredos) e envia `SettingsPatch`.

use std::path::Path;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use ts_rs::TS;

use crate::db::Db;
use crate::error::{CoreError, CoreResult};
use crate::events::EventSink;
use crate::logging;

pub const EVENT_CHANGED: &str = "settings://changed";

/// Ids dos perfis de saída (arquitetura §7).
pub const PROFILE_IDS: &[&str] = &[
    "original", "mp3_v0", "mp3_320", "aac_256", "opus_96", "flac",
];

pub const SPONSORBLOCK_CATEGORIES: &[&str] = &[
    "sponsor",
    "selfpromo",
    "interaction",
    "intro",
    "outro",
    "preview",
    "music_offtopic",
    "filler",
];

macro_rules! string_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident = $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
        #[ts(export)]
        pub enum $name {
            $( #[serde(rename = $text)] $variant, )+
        }
    };
}

string_enum!(Theme { Dark = "dark", Light = "light", System = "system" });
string_enum!(Language { PtBr = "pt-BR", En = "en" });
string_enum!(Transparency { Auto = "auto", Full = "full", Reduced = "reduced" });
string_enum!(SplitChapters { Ask = "ask", Always = "always", Never = "never" });
string_enum!(CookiesSource {
    None = "none",
    Firefox = "firefox",
    Chrome = "chrome",
    Edge = "edge",
    Brave = "brave",
    File = "file",
});
string_enum!(YtdlpChannel { Stable = "stable", Nightly = "nightly" });
string_enum!(JsRuntime {
    Auto = "auto",
    ManagedDeno = "managed-deno",
    SystemDeno = "system-deno",
    SystemNode = "system-node",
});
string_enum!(PotProvider { Auto = "auto", Always = "always", Off = "off" });

/// Quais chaves opcionais estão preenchidas (a UI nunca recebe os valores).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SecretsStatus {
    pub acoustid: bool,
    pub spotify: bool,
    pub discogs: bool,
    pub jamendo: bool,
}

macro_rules! define_settings {
    (
        public { $( $field:ident : $ty:ty = $default:expr ),+ $(,)? }
        secrets { $( $secret:ident ),+ $(,)? }
    ) => {
        /// Estado interno completo (inclui segredos). Nunca vai para a UI nem para logs.
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase", default)]
        pub struct Settings {
            $( pub $field: $ty, )+
            $( pub $secret: String, )+
        }

        impl Default for Settings {
            fn default() -> Self {
                Self {
                    $( $field: $default, )+
                    $( $secret: String::new(), )+
                }
            }
        }

        /// O que a UI recebe: todos os campos menos os segredos, mais `secretsStatus`.
        #[derive(Debug, Clone, PartialEq, Serialize, TS)]
        #[serde(rename_all = "camelCase")]
        #[ts(export)]
        pub struct SettingsView {
            $( pub $field: $ty, )+
            pub secrets_status: SecretsStatus,
        }

        /// Alteração parcial. `Some("")` limpa um segredo. Chaves desconhecidas são erro.
        #[derive(Debug, Clone, Default, Deserialize, TS)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        #[ts(export, optional_fields)]
        pub struct SettingsPatch {
            $( pub $field: Option<$ty>, )+
            $( pub $secret: Option<String>, )+
        }

        impl Settings {
            pub fn view(&self) -> SettingsView {
                SettingsView {
                    $( $field: self.$field.clone(), )+
                    secrets_status: self.secrets_status(),
                }
            }

            pub fn apply(&mut self, patch: SettingsPatch) {
                $( if let Some(value) = patch.$field { self.$field = value; } )+
                $( if let Some(value) = patch.$secret { self.$secret = value; } )+
            }

            /// Copia só os campos secretos de `other` (usado pelo `reset`).
            fn keep_secrets_from(&mut self, other: &Settings) {
                $( self.$secret = other.$secret.clone(); )+
            }

            fn secret_values(&self) -> Vec<&str> {
                vec![ $( self.$secret.as_str(), )+ ]
            }
        }
    };
}

define_settings! {
    public {
        output_dir: String = String::new(),
        file_template: String = "{albumartist}/{album}/{track:02} - {title}".to_string(),
        auto_organize: bool = true,
        default_profile: String = "original".to_string(),
        parallelism: u32 = 2,
        speed_limit_mbps: f64 = 0.0,
        queue_limit: u32 = 500,
        max_attempts: u32 = 3,
        fetch_metadata: bool = true,
        extract_title_from_video: bool = true,
        prefer_official_audio: bool = true,
        confidence_auto_apply: f64 = 0.85,
        confidence_review: f64 = 0.60,
        offline_mode: bool = false,
        fetch_artwork: bool = true,
        write_folder_cover: bool = true,
        fetch_lyrics: bool = true,
        write_lrc_file: bool = true,
        normalize_volume: bool = true,
        sponsorblock_remove: bool = false,
        sponsorblock_categories: Vec<String> = vec!["music_offtopic".to_string()],
        split_chapters: SplitChapters = SplitChapters::Ask,
        trim_silence: bool = false,
        playlist_pacing_seconds: u32 = 3,
        watch_library: bool = true,
        theme: Theme = Theme::Dark,
        language: Language = Language::PtBr,
        transparency: Transparency = Transparency::Auto,
        launch_at_startup: bool = false,
        start_minimized: bool = true,
        minimize_to_tray: bool = true,
        close_to_tray: bool = true,
        completion_notifications: bool = true,
        clipboard_watch: bool = false,
        global_shortcut: String = String::new(),
        weekly_self_test: bool = true,
        onboarding_completed: bool = false,
        cookies_source: CookiesSource = CookiesSource::None,
        cookies_file: String = String::new(),
        ytdlp_channel: YtdlpChannel = YtdlpChannel::Stable,
        js_runtime: JsRuntime = JsRuntime::Auto,
        auto_update_tools: bool = true,
        pot_provider: PotProvider = PotProvider::Auto,
        quality_target_kbps: u32 = 0,
        auto_upgrade: bool = false,
        artist_check_interval_hours: u32 = 24,
        auto_check_app_updates: bool = true,
        verify_lossless_on_import: bool = true,
    }
    secrets {
        acoustid_key,
        spotify_client_id,
        spotify_client_secret,
        discogs_token,
        jamendo_client_id,
    }
}

impl Settings {
    pub fn secrets_status(&self) -> SecretsStatus {
        SecretsStatus {
            acoustid: !self.acoustid_key.is_empty(),
            spotify: !self.spotify_client_id.is_empty() && !self.spotify_client_secret.is_empty(),
            discogs: !self.discogs_token.is_empty(),
            jamendo: !self.jamendo_client_id.is_empty(),
        }
    }
}

// ───────────────────────────── validação (§6) ─────────────────────────────

fn invalid(field: &str, message: impl Into<String>) -> CoreError {
    CoreError::invalid_i18n(message, format!("errors.settings.{field}"))
}

fn check_range<T: PartialOrd + std::fmt::Display>(
    field: &str,
    value: T,
    min: T,
    max: T,
) -> CoreResult<()> {
    if value < min || value > max {
        return Err(invalid(
            field,
            format!("{field} deve estar entre {min} e {max} (recebido {value})"),
        ));
    }
    Ok(())
}

const TEMPLATE_VARS: &[&str] = &[
    "artist",
    "albumartist",
    "album",
    "title",
    "track",
    "disc",
    "year",
    "genre",
    "channel",
    "source_id",
    "playlist",
    "playlist_index",
];
/// Variáveis que aceitam formato numérico com zeros à esquerda (`{track:02}`).
const TEMPLATE_PADDED_VARS: &[&str] = &["track", "disc", "playlist_index"];

/// Valida as variáveis de um modelo de nomes (arquitetura §13).
pub fn validate_file_template(template: &str) -> CoreResult<()> {
    if template.trim().is_empty() {
        return Err(invalid(
            "fileTemplate",
            "o modelo de nomes não pode ser vazio",
        ));
    }
    let mut rest = template;
    while let Some(open) = rest.find(['{', '}']) {
        if rest.as_bytes()[open] == b'}' {
            return Err(invalid("fileTemplate", "chave '}' sem '{' correspondente"));
        }
        let after = &rest[open + 1..];
        let close = after
            .find('}')
            .ok_or_else(|| invalid("fileTemplate", "chave '{' sem '}' correspondente"))?;
        let body = &after[..close];
        let (name, spec) = match body.split_once(':') {
            Some((name, spec)) => (name, Some(spec)),
            None => (body, None),
        };
        if !TEMPLATE_VARS.contains(&name) {
            return Err(invalid(
                "fileTemplate",
                format!("variável desconhecida no modelo: {{{body}}}"),
            ));
        }
        if let Some(spec) = spec {
            let ok = TEMPLATE_PADDED_VARS.contains(&name)
                && !spec.is_empty()
                && spec.len() <= 2
                && spec.chars().all(|c| c.is_ascii_digit());
            if !ok {
                return Err(invalid(
                    "fileTemplate",
                    format!("formato inválido no modelo: {{{body}}}"),
                ));
            }
        }
        rest = &after[close + 1..];
    }
    Ok(())
}

const SHORTCUT_MODIFIERS: &[&str] = &[
    "ctrl",
    "control",
    "alt",
    "option",
    "shift",
    "super",
    "meta",
    "cmd",
    "command",
    "cmdorcontrol",
    "commandorcontrol",
    "cmdorctrl",
];
const SHORTCUT_NAMED_KEYS: &[&str] = &[
    "space",
    "enter",
    "tab",
    "escape",
    "esc",
    "backspace",
    "delete",
    "insert",
    "home",
    "end",
    "pageup",
    "pagedown",
    "up",
    "down",
    "left",
    "right",
];

/// Validação sintática do acelerador (a F12 confere com o plugin de atalho global).
pub fn validate_shortcut(shortcut: &str) -> CoreResult<()> {
    if shortcut.is_empty() {
        return Ok(());
    }
    let parts: Vec<&str> = shortcut.split('+').map(str::trim).collect();
    let err = || {
        invalid(
            "globalShortcut",
            format!("atalho inválido: \"{shortcut}\" (exemplo: Ctrl+Shift+D)"),
        )
    };
    if parts.len() < 2 || parts.iter().any(|p| p.is_empty()) {
        return Err(err());
    }
    let (key, modifiers) = parts.split_last().expect("len >= 2");
    if !modifiers
        .iter()
        .all(|m| SHORTCUT_MODIFIERS.contains(&m.to_lowercase().as_str()))
    {
        return Err(err());
    }
    let lower = key.to_lowercase();
    let is_function_key = lower
        .strip_prefix('f')
        .and_then(|n| n.parse::<u8>().ok())
        .is_some_and(|n| (1..=24).contains(&n));
    let is_single = key.chars().count() == 1 && key.chars().all(|c| c.is_ascii_alphanumeric());
    if !(is_function_key || is_single || SHORTCUT_NAMED_KEYS.contains(&lower.as_str())) {
        return Err(err());
    }
    Ok(())
}

/// Valida o conjunto completo de configurações (regras cruzadas incluídas).
pub fn validate(settings: &Settings) -> CoreResult<()> {
    let s = settings;
    if !s.output_dir.is_empty() && !Path::new(&s.output_dir).is_absolute() {
        return Err(invalid(
            "outputDir",
            "a pasta de destino deve ser um caminho absoluto (ou vazia para o padrão)",
        ));
    }
    validate_file_template(&s.file_template)?;
    if !PROFILE_IDS.contains(&s.default_profile.as_str()) {
        return Err(invalid(
            "defaultProfile",
            format!("perfil desconhecido: {}", s.default_profile),
        ));
    }
    check_range("parallelism", s.parallelism, 1, 4)?;
    check_range("speedLimitMbps", s.speed_limit_mbps, 0.0, 1000.0)?;
    check_range("queueLimit", s.queue_limit, 10, 5000)?;
    check_range("maxAttempts", s.max_attempts, 1, 10)?;
    check_range("confidenceAutoApply", s.confidence_auto_apply, 0.5, 1.0)?;
    check_range("confidenceReview", s.confidence_review, 0.3, 0.95)?;
    if s.confidence_auto_apply <= s.confidence_review {
        return Err(invalid(
            "confidenceAutoApply",
            "confidenceAutoApply deve ser maior que confidenceReview",
        ));
    }
    if let Some(bad) = s
        .sponsorblock_categories
        .iter()
        .find(|c| !SPONSORBLOCK_CATEGORIES.contains(&c.as_str()))
    {
        return Err(invalid(
            "sponsorblockCategories",
            format!("categoria do SponsorBlock desconhecida: {bad}"),
        ));
    }
    check_range("playlistPacingSeconds", s.playlist_pacing_seconds, 0, 60)?;
    validate_shortcut(&s.global_shortcut)?;
    if s.cookies_source == CookiesSource::File && !Path::new(&s.cookies_file).is_file() {
        return Err(invalid(
            "cookiesFile",
            "o arquivo de cookies não existe (cookiesSource = file)",
        ));
    }
    if !matches!(s.quality_target_kbps, 0 | 160 | 256) {
        return Err(invalid(
            "qualityTargetKbps",
            "qualityTargetKbps deve ser 0, 160 ou 256",
        ));
    }
    check_range(
        "artistCheckIntervalHours",
        s.artist_check_interval_hours,
        6,
        168,
    )?;
    Ok(())
}

// ───────────────────────────── persistência ─────────────────────────────

fn to_map(settings: &Settings) -> Map<String, Value> {
    match serde_json::to_value(settings) {
        Ok(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

/// Lê as configurações do banco: padrões + valores gravados (valores inválidos são ignorados).
fn load(conn: &rusqlite::Connection) -> CoreResult<Settings> {
    let mut merged = to_map(&Settings::default());
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (key, raw) in rows {
        let Ok(value) = serde_json::from_str::<Value>(&raw) else {
            tracing::warn!(
                key,
                "configuração gravada com JSON inválido; usando o padrão"
            );
            continue;
        };
        if !merged.contains_key(&key) {
            continue;
        }
        let previous = merged.insert(key.clone(), value);
        if serde_json::from_value::<Settings>(Value::Object(merged.clone())).is_err() {
            tracing::warn!(
                key,
                "configuração gravada com valor inválido; usando o padrão"
            );
            if let Some(previous) = previous {
                merged.insert(key, previous);
            }
        }
    }
    Ok(serde_json::from_value(Value::Object(merged))?)
}

fn register_secrets(settings: &Settings) {
    logging::register_secret(&settings.cookies_file);
    for secret in settings.secret_values() {
        logging::register_secret(secret);
    }
}

pub struct SettingsService {
    db: Db,
    sink: Arc<dyn EventSink>,
    cache: Arc<RwLock<Settings>>,
}

impl SettingsService {
    pub async fn new(db: Db, sink: Arc<dyn EventSink>) -> CoreResult<Self> {
        let loaded = db.call(|conn| load(conn)).await?;
        register_secrets(&loaded);
        Ok(Self {
            db,
            sink,
            cache: Arc::new(RwLock::new(loaded)),
        })
    }

    /// Cópia do estado atual (com segredos; uso interno do backend).
    pub fn get(&self) -> Settings {
        self.cache.read().expect("cache de configurações").clone()
    }

    pub fn view(&self) -> SettingsView {
        self.get().view()
    }

    /// Valida, persiste chave a chave em transação, emite `settings://changed`.
    pub async fn update(&self, patch: SettingsPatch) -> CoreResult<Settings> {
        let cache = Arc::clone(&self.cache);
        let updated = self
            .db
            .call(move |conn| {
                let tx = conn.transaction()?;
                let current = load(&tx)?;
                let mut next = current.clone();
                next.apply(patch);
                validate(&next)?;
                if !next.output_dir.is_empty() {
                    std::fs::create_dir_all(&next.output_dir).map_err(|e| {
                        invalid(
                            "outputDir",
                            format!("não foi possível criar a pasta de destino: {e}"),
                        )
                    })?;
                }
                let before = to_map(&current);
                for (key, value) in to_map(&next) {
                    if before.get(&key) != Some(&value) {
                        tx.execute(
                            "INSERT INTO settings (key, value) VALUES (?1, ?2) \
                             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                            (&key, serde_json::to_string(&value)?),
                        )?;
                    }
                }
                tx.commit()?;
                *cache.write().expect("cache de configurações") = next.clone();
                Ok(next)
            })
            .await?;
        self.after_change(&updated);
        Ok(updated)
    }

    /// Volta tudo ao padrão, mantendo os segredos.
    pub async fn reset(&self) -> CoreResult<Settings> {
        let cache = Arc::clone(&self.cache);
        let updated = self
            .db
            .call(move |conn| {
                let tx = conn.transaction()?;
                let current = load(&tx)?;
                let mut next = Settings::default();
                next.keep_secrets_from(&current);
                tx.execute("DELETE FROM settings", [])?;
                let defaults = to_map(&Settings::default());
                for (key, value) in to_map(&next) {
                    if defaults.get(&key) != Some(&value) {
                        tx.execute(
                            "INSERT INTO settings (key, value) VALUES (?1, ?2)",
                            (&key, serde_json::to_string(&value)?),
                        )?;
                    }
                }
                tx.commit()?;
                *cache.write().expect("cache de configurações") = next.clone();
                Ok(next)
            })
            .await?;
        self.after_change(&updated);
        Ok(updated)
    }

    fn after_change(&self, settings: &Settings) {
        register_secrets(settings);
        match serde_json::to_value(settings.view()) {
            Ok(payload) => self.sink.emit(EVENT_CHANGED, payload),
            Err(e) => tracing::error!(error = %e, "não foi possível serializar SettingsView"),
        }
    }
}

#[cfg(test)]
mod tests;
