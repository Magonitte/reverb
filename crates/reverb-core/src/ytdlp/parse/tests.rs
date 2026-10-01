use super::*;

const REAL: &str = include_str!("../../../../../tests/fixtures/ytdlp/stdout-download.txt");

fn real_lines() -> Vec<&'static str> {
    REAL.lines().collect()
}

#[test]
fn as_quatro_linhas_reais_do_anexo() {
    let lines = real_lines();
    let progress: Vec<_> = lines
        .iter()
        .filter_map(|l| parse_progress_line(l))
        .collect();
    assert_eq!(progress.len(), 3);

    // 1ª: eta e speed nulos.
    assert_eq!(
        progress[0],
        ProgressUpdate {
            downloaded: 1024,
            total: Some(252182),
            speed: None,
            eta: None,
            finished: false
        }
    );
    // 2ª: eta 0 e velocidade real.
    assert_eq!(progress[1].eta, Some(0));
    assert!((progress[1].speed.unwrap() - 3071490.3189511322).abs() < 1e-6);
    assert_eq!(progress[1].downloaded, 3072);
    // 3ª: status finished (não é o fim do job).
    assert!(progress[2].finished);
    assert_eq!(progress[2].downloaded, 252182);

    let done = lines.iter().find_map(|l| parse_done_line(l)).unwrap();
    assert_eq!(done.id, "jNQXAC9IVRw");
    assert_eq!(done.title, "Me at the zoo");
    assert_eq!(done.filepath, "C:\\Users\\…\\out\\jNQXAC9IVRw.opus");
    assert_eq!(done.ext, "opus");
    assert_eq!(done.abr, Some(106.064));
    assert_eq!(done.acodec.as_deref(), Some("opus"));
    assert_eq!(done.format_id.as_deref(), Some("251"));
    assert_eq!(done.duration, Some(19.0));
}

#[test]
fn sem_total_bytes_usa_a_estimativa() {
    let line = r#"REVERB_PROGRESS {"status": "downloading", "downloaded_bytes": 500, "total_bytes_estimate": 1000.5, "speed": 10.0, "eta": 3}"#;
    let update = parse_progress_line(line).unwrap();
    assert_eq!(update.total, Some(1000));
    assert_eq!(update.eta, Some(3));
}

#[test]
fn sem_nenhum_total_o_progresso_e_indeterminado() {
    let line = r#"REVERB_PROGRESS {"status": "downloading", "downloaded_bytes": 500, "eta": null, "speed": null}"#;
    let update = parse_progress_line(line).unwrap();
    assert_eq!(update.total, None);
    assert_eq!((update.speed, update.eta), (None, None));
    assert!(!update.finished);
}

#[test]
fn eta_fracionario_e_arredondado_para_baixo() {
    let line = r#"REVERB_PROGRESS {"status": "downloading", "downloaded_bytes": 1, "total_bytes": 2, "eta": 12.7}"#;
    assert_eq!(parse_progress_line(line).unwrap().eta, Some(12));
}

#[test]
fn linhas_lixo_sao_ignoradas() {
    for line in [
        "",
        "[download] 50% of 3MiB",
        "REVERB_PROGRESS",
        "REVERB_PROGRESS não é json",
        "REVERB_PROGRESS [1, 2]",
        "REVERB_PROGRESS {\"downloaded_bytes\": ",
        "XREVERB_PROGRESS {}",
        "  REVERB_PROGRESS {}",
    ] {
        assert_eq!(parse_progress_line(line), None, "{line:?}");
        assert_eq!(parse_done_line(line), None, "{line:?}");
    }
    assert_eq!(parse_done_line("REVERB_DONE {\"id\": \"x\"}"), None);
}

#[test]
fn titulo_com_emoji_e_acentos_sobrevive() {
    let line = r#"REVERB_DONE {"id": "abc", "title": "Música 🎵 ação – 日本語 \"aspas\"", "filepath": "/tmp/x/abc.m4a", "ext": "m4a", "abr": null, "acodec": "mp4a.40.2", "format_id": "140", "duration": 214.5}"#;
    let done = parse_done_line(line).unwrap();
    assert_eq!(done.title, "Música 🎵 ação – 日本語 \"aspas\"");
    assert_eq!(done.abr, None);
    assert_eq!(done.duration, Some(214.5));
}

#[test]
fn done_aceita_fim_de_linha_do_windows() {
    let line =
        "REVERB_DONE {\"id\": \"a\", \"title\": \"t\", \"filepath\": \"f\", \"ext\": \"opus\"}\r";
    assert!(parse_done_line(line).is_some());
}
