use super::*;

fn lines(text: &str) -> Vec<String> {
    text.lines().map(str::to_string).collect()
}

fn kind(text: &str) -> ErrorKind {
    classify_stderr(&lines(text))
}

#[test]
fn stderr_real_gravado_de_video_indisponivel() {
    let real = include_str!("../../../../../tests/fixtures/ytdlp/stderr-unavailable.txt");
    assert_eq!(kind(real), ErrorKind::Unavailable);
}

#[test]
fn tabela_do_plano() {
    let casos = [
        (
            "ERROR: [youtube] x: Video unavailable",
            ErrorKind::Unavailable,
        ),
        (
            "ERROR: [youtube] x: This video is unavailable",
            ErrorKind::Unavailable,
        ),
        (
            "ERROR: [youtube] x: Private video. Sign in if you've been granted access",
            ErrorKind::Unavailable,
        ),
        (
            "ERROR: This video has been removed by the uploader",
            ErrorKind::Unavailable,
        ),
        (
            "ERROR: Join this channel to get access to members-only content",
            ErrorKind::Unavailable,
        ),
        ("ERROR: This video is not available", ErrorKind::Unavailable),
        (
            "ERROR: unable to download video data: HTTP Error 404: Not Found",
            ErrorKind::Unavailable,
        ),
        (
            "ERROR: [youtube] x: Sign in to confirm your age. This video may be inappropriate",
            ErrorKind::AgeRestricted,
        ),
        (
            "ERROR: This video is age-restricted",
            ErrorKind::AgeRestricted,
        ),
        (
            "ERROR: [youtube] x: Sign in to confirm you're not a bot",
            ErrorKind::BotCheck,
        ),
        (
            "ERROR: [youtube] x: Sign in to confirm you’re not a bot",
            ErrorKind::BotCheck,
        ),
        ("Please confirm you are not a robot", ErrorKind::BotCheck),
        (
            "ERROR: The uploader has not made this video available in your country",
            ErrorKind::GeoBlocked,
        ),
        ("ERROR: geo restriction applies", ErrorKind::GeoBlocked),
        (
            "ERROR: [youtube] x: Video unavailable. This video is not available in your country",
            ErrorKind::GeoBlocked,
        ),
        (
            "ERROR: unable to write data: [Errno 28] No space left on device",
            ErrorKind::Disk,
        ),
        (
            "ERROR: [Errno 13] Permission denied: 'C:\\x.part'",
            ErrorKind::Disk,
        ),
        ("OSError: Access is denied", ErrorKind::Disk),
        ("ERROR: ENOSPC", ErrorKind::Disk),
        (
            "ERROR: ffmpeg not found. Please install or provide the path using --ffmpeg-location",
            ErrorKind::Ffmpeg,
        ),
        ("ERROR: ffprobe not found", ErrorKind::Ffmpeg),
        (
            "ERROR: Postprocessing: Conversion failed!",
            ErrorKind::Ffmpeg,
        ),
        (
            "ERROR: Postprocessing: ffmpeg exited with Error",
            ErrorKind::Ffmpeg,
        ),
        (
            "ERROR: [youtube] x: Unable to extract initial player response",
            ErrorKind::Extractor,
        ),
        (
            "WARNING: [youtube] nsig extraction failed: Some formats may be missing",
            ErrorKind::Extractor,
        ),
        ("ERROR: Signature extraction failed", ErrorKind::Extractor),
        (
            "ERROR: [youtube] x: Requested format is not available. Use --list-formats",
            ErrorKind::Extractor,
        ),
        (
            "ERROR: unable to download video data: HTTP Error 403: Forbidden",
            ErrorKind::Extractor,
        ),
        (
            "WARNING: [youtube] [jsc] Error solving challenge",
            ErrorKind::Extractor,
        ),
        (
            "ERROR: [youtube] x: No video formats found!",
            ErrorKind::Extractor,
        ),
        (
            "ERROR: Unable to download webpage: <urlopen error>",
            ErrorKind::Network,
        ),
        ("ERROR: The read operation timed out", ErrorKind::Network),
        ("ERROR: Connection reset by peer", ErrorKind::Network),
        ("ERROR: Connection refused", ErrorKind::Network),
        ("ERROR: Connection aborted.", ErrorKind::Network),
        (
            "ERROR: [Errno 11001] getaddrinfo failed",
            ErrorKind::Network,
        ),
        (
            "ERROR: Temporary failure in name resolution",
            ErrorKind::Network,
        ),
        (
            "ERROR: HTTP Error 503: Service Unavailable",
            ErrorKind::Network,
        ),
        (
            "ERROR: IncompleteRead(1024 bytes read, 2048 more expected)",
            ErrorKind::Network,
        ),
        ("ERROR: algo completamente diferente", ErrorKind::Unknown),
        ("", ErrorKind::Unknown),
    ];
    assert!(casos.len() >= 18);
    for (text, expected) in casos {
        assert_eq!(kind(text), expected, "{text}");
    }
}

#[test]
fn e_insensivel_a_maiusculas() {
    assert_eq!(kind("error: VIDEO UNAVAILABLE"), ErrorKind::Unavailable);
    assert_eq!(kind("error: CONNECTION RESET"), ErrorKind::Network);
}

#[test]
fn primeira_regra_da_tabela_vence_entre_varias_linhas() {
    // Aviso de extrator + erro definitivo de indisponibilidade ⇒ indisponível.
    let tail = vec![
        "WARNING: [youtube] Some formats may be missing".to_string(),
        "ERROR: [youtube] x: Video unavailable".to_string(),
    ];
    assert_eq!(classify_stderr(&tail), ErrorKind::Unavailable);
}

#[test]
fn download_error_usa_a_ultima_linha_error_como_mensagem() {
    let tail =
        lines("WARNING: algo\nERROR: [youtube] x: Video unavailable\n[download] fim sem sentido");
    let err = DownloadError::from_stderr(tail.clone(), "falhou");
    assert_eq!(err.kind, ErrorKind::Unavailable);
    assert_eq!(err.message, "ERROR: [youtube] x: Video unavailable");
    assert_eq!(err.stderr_tail, tail);

    let vazio = DownloadError::from_stderr(Vec::new(), "falhou");
    assert_eq!(
        (vazio.kind, vazio.message.as_str()),
        (ErrorKind::Unknown, "falhou")
    );
}

#[test]
fn retentaveis_sao_network_unknown_e_ffmpeg() {
    use ErrorKind::*;
    for kind in [Network, Unknown, Ffmpeg] {
        assert!(kind.is_retryable());
    }
    for kind in [
        Cancelled,
        Unavailable,
        AgeRestricted,
        BotCheck,
        GeoBlocked,
        Disk,
        Extractor,
    ] {
        assert!(!kind.is_retryable());
    }
}

#[test]
fn serializa_em_snake_case() {
    assert_eq!(
        serde_json::to_value(ErrorKind::AgeRestricted).unwrap(),
        "age_restricted"
    );
    assert_eq!(ErrorKind::BotCheck.as_str(), "bot_check");
}

#[test]
fn cookie_failures_are_actionable_and_not_retried() {
    for (stderr, expected) in [
        (
            "ERROR: could not find firefox cookies database",
            ErrorKind::BrowserNotFound,
        ),
        (
            "ERROR: Could not copy Chrome cookie database. Permission denied",
            ErrorKind::CookiesLocked,
        ),
        (
            "ERROR: Failed to decrypt cookies with DPAPI",
            ErrorKind::CookiesDecrypt,
        ),
    ] {
        assert_eq!(kind(stderr), expected);
        assert!(!expected.is_retryable());
    }
}
