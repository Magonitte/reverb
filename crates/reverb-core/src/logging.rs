//! Logs com `tracing`: arquivo diário em `logs/` (14 dias) e redação de segredos (arquitetura §18).

use std::io::{self, Write};
use std::path::Path;
use std::sync::{LazyLock, RwLock};

use regex::Regex;
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};

pub const LOG_FILE_PREFIX: &str = "reverb.log";
pub const LOG_RETENTION: usize = 14;
const REDACTED: &str = "***";
/// Segredos curtos demais (ex.: "a") apagariam texto comum; abaixo disso não são registrados.
const MIN_SECRET_LEN: usize = 4;

static KNOWN_SECRETS: RwLock<Vec<String>> = RwLock::new(Vec::new());

static SENSITIVE_PAIR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)(\b(?:api[_-]?key|key|access_token|refresh_token|token|client_secret|sig|signature)=)[^&\s"']+"#,
    )
    .expect("regex de redação")
});
static AUTH_HEADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\b(authorization|cookie|set-cookie)(\s*[:=]\s*)[^\r\n"]+"#)
        .expect("regex de redação")
});

/// Registra um valor secreto conhecido (chave de API etc.) para ser sempre mascarado.
pub fn register_secret(value: &str) {
    if value.len() < MIN_SECRET_LEN {
        return;
    }
    let mut secrets = KNOWN_SECRETS.write().expect("lista de segredos");
    if !secrets.iter().any(|s| s == value) {
        secrets.push(value.to_string());
    }
}

/// Mascara query strings sensíveis, cabeçalhos de autorização/cookies e segredos conhecidos.
pub fn redact(text: &str) -> String {
    let mut out = SENSITIVE_PAIR
        .replace_all(text, format!("${{1}}{REDACTED}"))
        .into_owned();
    out = AUTH_HEADER
        .replace_all(&out, format!("${{1}}${{2}}{REDACTED}"))
        .into_owned();
    for secret in KNOWN_SECRETS.read().expect("lista de segredos").iter() {
        if out.contains(secret.as_str()) {
            out = out.replace(secret.as_str(), REDACTED);
        }
    }
    out
}

/// Writer que redige cada bloco escrito. O `fmt` do tracing escreve um evento por chamada.
pub struct RedactingWriter<W: Write>(W);

impl<W: Write> Write for RedactingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let text = String::from_utf8_lossy(buf);
        self.0.write_all(redact(&text).as_bytes())?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

/// Mantém só os `keep` arquivos de log mais recentes (o sufixo de data ordena por nome).
pub fn prune_old_logs(dir: &Path, keep: usize) -> io::Result<usize> {
    let mut files: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().starts_with(LOG_FILE_PREFIX))
        .map(|e| e.path())
        .collect();
    files.sort();
    let excess = files.len().saturating_sub(keep);
    for path in &files[..excess] {
        std::fs::remove_file(path)?;
    }
    Ok(excess)
}

/// Mantém o worker de escrita vivo; descarte-o só ao encerrar o app.
pub struct LogGuard(#[allow(dead_code)] WorkerGuard);

#[derive(Clone)]
struct MakeRedacted(NonBlocking);

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for MakeRedacted {
    type Writer = RedactingWriter<NonBlocking>;

    fn make_writer(&'a self) -> Self::Writer {
        RedactingWriter(self.0.clone())
    }
}

/// Inicializa o log global: arquivo diário em `log_dir`, retenção de 14 arquivos, redação.
/// Em debug também escreve no stderr (redigido). Chame uma única vez por processo.
pub fn init(log_dir: &Path) -> io::Result<LogGuard> {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::{fmt, EnvFilter};

    std::fs::create_dir_all(log_dir)?;
    let appender = tracing_appender::rolling::daily(log_dir, LOG_FILE_PREFIX);
    let (writer, guard) = tracing_appender::non_blocking(appender);
    prune_old_logs(log_dir, LOG_RETENTION)?;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(MakeRedacted(writer));
    let stderr_layer =
        cfg!(debug_assertions).then(|| fmt::layer().with_writer(|| RedactingWriter(io::stderr())));

    tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(stderr_layer)
        .try_init()
        .map_err(io::Error::other)?;
    Ok(LogGuard(guard))
}

#[cfg(test)]
mod tests;
