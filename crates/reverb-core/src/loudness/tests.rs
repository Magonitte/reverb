use super::*;

const FIXTURE: &str = include_str!("../../../../tests/fixtures/ffmpeg/ebur128-pink-noise.txt");

fn summary(i: &str, peak: &str) -> String {
    format!("[Parsed_ebur128_0 @ 0x123] Summary:\n\n  Integrated loudness:\n    I: {i} LUFS\n    Threshold: -33.0 LUFS\n\n  True peak:\n    Peak: {peak} dBFS\n")
}

#[test]
fn t3a_extrai_i_e_true_peak_exatos_do_resumo_real() {
    assert!(FIXTURE.contains("Summary:"));
    let measured = parse_summary(FIXTURE).unwrap();
    assert_eq!(measured.integrated_lufs, -25.1);
    assert_eq!(measured.true_peak_dbfs, -12.6);
}

#[test]
fn ignora_frames_thresholds_e_sample_peak_e_usa_ultimo_resumo() {
    let output = format!(
        "{}\n[Parsed_ebur128] t: 0.2 I: -1.0 LUFS Peak: 20.0 dBFS\n{}",
        summary("-70.0", "-inf"),
        summary("-14.2", "-1.2").replace(
            "  True peak:",
            "  Sample peak:\n    Peak: -2.0 dBFS\n  True peak:"
        )
    );
    let measured = parse_summary(&output).unwrap();
    assert_eq!(measured.integrated_lufs, -14.2);
    assert_eq!(measured.true_peak_dbfs, -1.2);
}

#[test]
fn tolera_crlf_espacos_e_prefixos_do_ffmpeg() {
    let output = summary("-1.42e1", "+0.5")
        .lines()
        .map(|line| format!("[info] [Parsed_ebur128 @ 0x456] {line}\r\n"))
        .collect::<String>();
    let measured = parse_summary(&output).unwrap();
    assert_eq!(measured.integrated_lufs, -14.2);
    assert_eq!(measured.true_peak_dbfs, 0.5);
}

#[test]
fn nao_usa_resumo_anterior_quando_o_ultimo_e_incompleto() {
    let output = format!(
        "{}\nSummary:\nIntegrated loudness:\nI: -20.0 LUFS",
        summary("-14.2", "-1.2")
    );
    assert!(parse_summary(&output).is_err());
}

#[test]
fn rejeita_resumos_ausentes_incompletos_ou_com_unidade_errada() {
    for output in [
        String::new(),
        "I: -14.2 LUFS\nPeak: -1.2 dBFS".into(),
        "Summary:\nTrue peak:\nPeak: -1.2 dBFS".into(),
        summary("-14.2", "-1.2").replace("I: -14.2 LUFS", "Threshold: -14.2 LUFS"),
        summary("-14.2", "-1.2").replace("Peak: -1.2 dBFS", "Peak: -1.2 dB"),
        summary("-14.2", "-1.2").replace("I: -14.2 LUFS", "I: -14.2 dB"),
        summary("invalid", "-1.2"),
    ] {
        assert_eq!(
            parse_summary(&output).unwrap_err().kind(),
            "loudness_parse",
            "{output}"
        );
    }
}

#[test]
fn rejeita_nan_infinito_e_overflow_sem_saturar_ganho() {
    for (i, peak) in [
        ("NaN", "-1"),
        ("inf", "-1"),
        ("-inf", "-1"),
        ("-18", "NaN"),
        ("-18", "inf"),
        ("-18", "9999"),
        ("-200", "-1"),
    ] {
        assert!(
            parse_summary(&summary(i, peak)).is_err(),
            "I: {i}, peak: {peak}"
        );
    }
}

#[test]
fn t3c_tags_com_precisao_e_r128_inteiro() {
    let gain = parse_summary(&summary("-14.2", "-1.2"))
        .unwrap()
        .replay_gain()
        .unwrap();
    assert!((gain.track_gain_db - (-3.8)).abs() < 1e-12);
    assert!((gain.track_peak - 10.0_f64.powf(-1.2 / 20.0)).abs() < 1e-12);
    assert_eq!(gain.track_gain_tag(), "-3.80 dB");
    assert_eq!(gain.track_peak_tag(), "0.870964");
    assert_eq!(gain.r128_track_gain, -2253);
    assert_eq!(gain.r128_track_gain_tag(), "-2253");
    assert_eq!(
        gain.r128_track_gain,
        ((-23.0 - (-14.2_f64)) * 256.0).round() as i16
    );
}

#[test]
fn ganho_positivo_zero_e_pico_acima_de_um() {
    let gain = parse_summary(FIXTURE).unwrap().replay_gain().unwrap();
    assert_eq!(gain.track_gain_tag(), "7.10 dB");
    assert_eq!(gain.r128_track_gain, 538);
    let gain = parse_summary(&summary("-18.0", "0.0"))
        .unwrap()
        .replay_gain()
        .unwrap();
    assert_eq!(gain.track_gain_tag(), "0.00 dB");
    assert_eq!(gain.track_peak_tag(), "1.000000");
    assert_eq!(gain.r128_track_gain, -1280);
    let gain = parse_summary(&summary("-17.999", "6.0"))
        .unwrap()
        .replay_gain()
        .unwrap();
    assert_eq!(gain.track_gain_tag(), "0.00 dB");
    assert_eq!(gain.track_peak_tag(), "1.995262");
}

#[test]
fn pico_menos_infinito_vira_amplitude_zero() {
    let gain = parse_summary(&summary("-70.0", "-inf"))
        .unwrap()
        .replay_gain()
        .unwrap();
    assert_eq!(gain.track_peak_tag(), "0.000000");
    assert_eq!(gain.track_gain_tag(), "52.00 dB");
    assert_eq!(gain.r128_track_gain, 12032);
}

#[test]
fn argumentos_preservam_caminho_com_espacos_e_acentos_sem_saida_audio() {
    let file = Path::new("pasta com espaços/música.wav");
    let args = analyze_args(file);
    let input = args.iter().position(|a| a == "-i").unwrap();
    assert_eq!(args[input + 1], file.to_string_lossy());
    assert_eq!(
        &args[args.len() - 5..],
        ["-af", "ebur128=peak=true", "-f", "null", "-"]
    );
    assert!(args.iter().any(|a| a == "-nostdin"));
    assert!(!args.iter().any(|a| a == "-y"));
}
