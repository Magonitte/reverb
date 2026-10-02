use crate::tools::{detect_js_runtime, run_capture, Detection, Tool};
use crate::{CoreResult, Db, SettingsService, ToolsManager};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum DiagnosticLevel {
    Ok,
    Warning,
    Error,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DiagnosticItem {
    pub id: String,
    pub level: DiagnosticLevel,
    pub detail: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DiagnosticReport {
    #[ts(type = "number")]
    pub created_at: i64,
    pub items: Vec<DiagnosticItem>,
}
impl DiagnosticReport {
    pub fn has_errors(&self) -> bool {
        self.items
            .iter()
            .any(|item| item.level == DiagnosticLevel::Error)
    }
    fn add(&mut self, id: &str, level: DiagnosticLevel, detail: impl Into<String>) {
        self.items.push(DiagnosticItem {
            id: id.into(),
            level,
            detail: crate::logging::redact(&detail.into()),
        });
    }
}

pub struct DiagnosticProgram {
    pub id: String,
    pub path: Option<PathBuf>,
    pub args: Vec<String>,
}
pub struct DiagnosticConfig {
    pub programs: Vec<DiagnosticProgram>,
    pub runtime: Option<String>,
    pub simulate_args: Vec<String>,
    pub output_dir: PathBuf,
    pub free_bytes: Option<u64>,
    pub latest: Result<(String, bool), String>,
    pub pot_detail: String,
    pub pot_warning: bool,
    pub extra_env: Vec<(String, String)>,
}
pub async fn inspect(db: &Db, config: DiagnosticConfig) -> CoreResult<DiagnosticReport> {
    let mut report = DiagnosticReport {
        created_at: crate::queue::repo::now(),
        items: vec![],
    };
    for program in &config.programs {
        let output = match &program.path {
            Some(path) => {
                run_capture(
                    path,
                    &program.args,
                    |command| {
                        command.envs(config.extra_env.iter().cloned());
                    },
                    Duration::from_secs(15),
                )
                .await
            }
            None => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Tool not installed",
            )),
        };
        match output {
            Ok(output) if output.success => {
                report.add(&program.id, DiagnosticLevel::Ok, output.stdout.trim())
            }
            Ok(output) => report.add(&program.id, DiagnosticLevel::Error, output.stderr),
            Err(error) => report.add(&program.id, DiagnosticLevel::Error, error.to_string()),
        }
    }
    report.add(
        "runtime",
        if config.runtime.is_some() {
            DiagnosticLevel::Ok
        } else {
            DiagnosticLevel::Error
        },
        config
            .runtime
            .clone()
            .unwrap_or_else(|| "No supported JavaScript runtime".into()),
    );
    match config.latest {
        Ok((version, update)) => report.add(
            "ytdlpVersion",
            if update {
                DiagnosticLevel::Warning
            } else {
                DiagnosticLevel::Ok
            },
            version,
        ),
        Err(error) => report.add("ytdlpVersion", DiagnosticLevel::Warning, error),
    }
    if let Some(program) = config
        .programs
        .iter()
        .find(|program| program.id == "ytdlp")
        .and_then(|program| program.path.as_ref())
        .filter(|_| config.runtime.is_some())
    {
        let output = run_capture(
            program,
            &config.simulate_args,
            |command| {
                command.envs(config.extra_env.iter().cloned());
            },
            Duration::from_secs(90),
        )
        .await;
        match output {
            Ok(output)
                if output.success && !output.stderr.contains("No supported JavaScript runtime") =>
            {
                report.add("simulate", DiagnosticLevel::Ok, "FX1 --simulate completed")
            }
            Ok(output) => report.add("simulate", DiagnosticLevel::Error, output.stderr),
            Err(error) => report.add("simulate", DiagnosticLevel::Error, error.to_string()),
        }
    } else {
        report.add(
            "simulate",
            DiagnosticLevel::Error,
            "Cannot simulate without yt-dlp and a JavaScript runtime",
        );
    }
    let output = config.output_dir;
    let (write, space) = tokio::task::spawn_blocking(move || {
        let write = (|| -> std::io::Result<()> {
            std::fs::create_dir_all(&output)?;
            let mut file = tempfile::NamedTempFile::new_in(&output)?;
            std::io::Write::write_all(&mut file, b"Reverb diagnostics")?;
            file.as_file().sync_all()
        })();
        let space = config
            .free_bytes
            .map(Ok)
            .unwrap_or_else(|| fs2::available_space(&output));
        (write, space)
    })
    .await
    .map_err(|error| crate::CoreError::Internal(error.to_string()))?;
    match write {
        Ok(()) => report.add(
            "outputWrite",
            DiagnosticLevel::Ok,
            "Output directory is writable",
        ),
        Err(error) => report.add("outputWrite", DiagnosticLevel::Error, error.to_string()),
    }
    match space {
        Ok(bytes) => report.add(
            "diskSpace",
            if bytes > 500 * 1024 * 1024 {
                DiagnosticLevel::Ok
            } else {
                DiagnosticLevel::Warning
            },
            format!("{} MiB available", bytes / 1024 / 1024),
        ),
        Err(error) => report.add("diskSpace", DiagnosticLevel::Warning, error.to_string()),
    }
    let integrity = db
        .call(|conn| {
            Ok(conn.query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))?)
        })
        .await?;
    report.add(
        "database",
        if integrity == "ok" {
            DiagnosticLevel::Ok
        } else {
            DiagnosticLevel::Error
        },
        integrity,
    );
    report.add(
        "pot",
        if config.pot_warning {
            DiagnosticLevel::Warning
        } else {
            DiagnosticLevel::Ok
        },
        config.pot_detail,
    );
    Ok(report)
}

pub async fn run(
    db: &Db,
    settings: Arc<SettingsService>,
    tools: Arc<ToolsManager>,
) -> CoreResult<DiagnosticReport> {
    let _guard = tools.acquire_run().await;
    let current = settings.get();
    let runtime = match detect_js_runtime(
        current.js_runtime,
        &std::env::var_os("PATH").unwrap_or_default(),
        tools.resolve(Tool::Deno).ok(),
    )
    .await
    {
        Ok(Detection::Found(choice)) => Some(choice.js_runtime_arg()),
        _ => None,
    };
    let latest = tools
        .check_updates(false)
        .await
        .map_err(|error| error.to_string())
        .and_then(|updates| {
            updates
                .into_iter()
                .find(|info| info.tool == Tool::Ytdlp)
                .map(|info| (info.latest, info.update_available))
                .ok_or_else(|| "No yt-dlp version available".into())
        });
    let pot_installed = tools.bgutil_paths().is_some();
    let pot_active = tools.pot_policy().is_active(current.pot_provider).await?;
    let until = tools.pot_policy().auto_until().await?;
    let status = tools.status().await?;
    let version = status
        .iter()
        .find(|status| status.tool == Tool::Bgutil)
        .and_then(|status| status.version.clone());
    let mut args = vec![
        "--ignore-config".into(),
        "--color".into(),
        "never".into(),
        "--js-runtimes".into(),
        runtime.clone().unwrap_or_default(),
        "--simulate".into(),
        "--no-playlist".into(),
        "--remote-components".into(),
        "ejs:github".into(),
    ];
    if let Some(pot) = tools.pot_args().await {
        args.extend(pot);
    }
    if let Ok(ffmpeg) = tools.resolve(Tool::Ffmpeg) {
        args.extend([
            "--ffmpeg-location".into(),
            ffmpeg.to_string_lossy().into_owned(),
        ]);
    }
    match current.cookies_source {
        crate::settings::CookiesSource::None => {}
        crate::settings::CookiesSource::File => {
            args.extend(["--cookies".into(), current.cookies_file.clone()])
        }
        source => args.extend([
            "--cookies-from-browser".into(),
            serde_json::to_value(source)?
                .as_str()
                .unwrap_or_default()
                .into(),
        ]),
    }
    args.extend([
        "--".into(),
        "https://www.youtube.com/watch?v=jNQXAC9IVRw".into(),
    ]);
    let config=DiagnosticConfig{programs:vec![DiagnosticProgram{id:"ytdlp".into(),path:tools.resolve(Tool::Ytdlp).ok(),args:vec!["--version".into()]},DiagnosticProgram{id:"ffmpeg".into(),path:tools.resolve(Tool::Ffmpeg).ok(),args:vec!["-version".into()]},DiagnosticProgram{id:"ffprobe".into(),path:tools.resolve_ffprobe().ok(),args:vec!["-version".into()]}],runtime,simulate_args:args,output_dir:crate::paths::resolve_output_dir(&current),free_bytes:None,latest,pot_detail:format!("mode={:?}; installed={pot_installed}; version={version:?}; active={pot_active}; server={}; autoUntil={until:?}",current.pot_provider,tools.pot_server_running().await),pot_warning:pot_active&&!pot_installed,extra_env:vec![]};
    let report = inspect(db, config).await?;
    db.kv_set("diagnostics.last", &serde_json::to_string(&report)?)
        .await?;
    Ok(report)
}
pub fn weekly_due(last: Option<i64>, now: i64) -> bool {
    last.is_none_or(|last| now.saturating_sub(last) >= 7 * 86400)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn t5_fake_tools_missing_runtime_low_space_and_ok() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open_in_memory().unwrap();
        for (runtime, bytes, errors, warnings) in [
            (None, 1024 * 1024 * 1024, true, false),
            (Some("fake"), 1, false, true),
            (Some("fake"), 1024 * 1024 * 1024, false, false),
        ] {
            let report = inspect(
                &db,
                DiagnosticConfig {
                    programs: vec![DiagnosticProgram {
                        id: "ytdlp".into(),
                        path: Some(crate::tools::testutil::fake_tool_path()),
                        args: vec!["--version".into()],
                    }],
                    runtime: runtime.map(str::to_owned),
                    simulate_args: vec!["--exit".into(), "0".into()],
                    output_dir: dir.path().to_owned(),
                    free_bytes: Some(bytes),
                    latest: Ok(("latest".into(), false)),
                    pot_detail: "auto; inactive".into(),
                    pot_warning: false,
                    extra_env: vec![],
                },
            )
            .await
            .unwrap();
            assert_eq!(report.has_errors(), errors);
            assert_eq!(
                report
                    .items
                    .iter()
                    .any(|item| item.level == DiagnosticLevel::Warning),
                warnings
            );
        }
    }
    #[test]
    fn weekly_interval_respects_boundary() {
        assert!(weekly_due(None, 1));
        assert!(!weekly_due(Some(1), 7 * 86400));
        assert!(weekly_due(Some(1), 7 * 86400 + 1));
    }
}
