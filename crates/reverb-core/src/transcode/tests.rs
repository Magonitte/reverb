use std::path::Path;

use super::*;
use crate::profiles::{profile, profile_for_source};

#[test]
fn parse_probe_le_codec_bitrate_duracao_e_canais() {
    let json = r#"{
        "streams":[{"codec_type":"audio","codec_name":"opus","sample_rate":"48000","channels":2,"bit_rate":"160123","duration":"10.007"}],
        "format":{"duration":"10.01","bit_rate":"165000"}
    }"#;
    let info = parse_probe(json).unwrap();
    assert_eq!(info.codec, "opus");
    assert_eq!(info.bitrate_kbps, Some(160));
    assert!((info.duration_s - 10.007).abs() < 1e-9);
    assert_eq!(info.sample_rate, Some(48000));
    assert_eq!(info.channels, Some(2));
}

#[test]
fn parse_probe_cai_para_o_format_quando_o_stream_nao_informa() {
    let json = r#"{
        "streams":[{"codec_type":"video","codec_name":"mjpeg"},{"codec_type":"audio","codec_name":"flac","channels":2}],
        "format":{"duration":"9.5","bit_rate":"900000"}
    }"#;
    let info = parse_probe(json).unwrap();
    assert_eq!(info.codec, "flac");
    assert_eq!(info.bitrate_kbps, Some(900));
    assert_eq!(info.duration_s, 9.5);
    assert_eq!(info.sample_rate, None);
}

#[test]
fn parse_probe_rejeita_entradas_invalidas() {
    for json in [
        "não é json",
        "{}",
        r#"{"streams":[{"codec_type":"video"}],"format":{}}"#,
        r#"{"streams":[{"codec_type":"audio","codec_name":"mp3"}],"format":{}}"#,
    ] {
        let err = parse_probe(json).unwrap_err();
        assert_eq!(err.kind, ErrorKind::Ffmpeg, "{json}");
    }
}

#[test]
fn progresso_em_percentual() {
    assert_eq!(
        parse_progress_percent("out_time_us=5000000", 10.0),
        Some(50)
    );
    assert_eq!(
        parse_progress_percent("out_time_ms=2500000", 10.0),
        Some(25)
    );
    assert_eq!(
        parse_progress_percent("out_time_us=99999999999", 10.0),
        Some(100)
    );
    assert_eq!(parse_progress_percent("out_time_us=N/A", 10.0), None);
    assert_eq!(parse_progress_percent("out_time_us=-5", 10.0), None);
    assert_eq!(parse_progress_percent("out_time_us=5", 0.0), None);
    assert_eq!(parse_progress_percent("bitrate=128kbits/s", 10.0), None);
    assert_eq!(parse_progress_percent("progress=continue", 10.0), None);
    assert_eq!(parse_progress_percent("progress=end", 10.0), Some(100));
}

#[test]
fn argumentos_da_conversao_seguem_o_plano() {
    let args = convert_args(
        Path::new("/tmp/a b/in.opus"),
        Path::new("/tmp/a b/out.mp3"),
        &profile("mp3_320").unwrap().resolve("opus"),
    );
    assert_eq!(
        args.join(" "),
        "-hide_banner -nostdin -y -i /tmp/a b/in.opus -map 0:a:0 -vn -map_metadata -1 \
         -c:a libmp3lame -b:a 320k -progress pipe:1 -nostats /tmp/a b/out.mp3"
    );
    assert!(
        args.contains(&"/tmp/a b/in.opus".to_string()),
        "caminho é um só argumento"
    );
    let fallback = convert_args(Path::new("i"), Path::new("o"), &profile_for_source("webm"));
    assert!(fallback.join(" ").contains("-c:a libopus -b:a 160k"));
}
