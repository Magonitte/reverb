//! T2 — snapshots dos argumentos. Cada argumento vira uma linha do snapshot, então um caminho com
//! espaços só aparece como **uma** linha quando é um único argumento.

use super::*;

const URL: &str = "https://www.youtube.com/watch?v=jNQXAC9IVRw";

fn ctx() -> YtDlpContext {
    YtDlpContext {
        ytdlp_path: PathBuf::from("/tools/yt-dlp"),
        js_runtime_arg: "deno:/tools/deno".to_string(),
        ffmpeg_dir: PathBuf::from("/tools/ffmpeg"),
        cookies: None,
        limit_rate_mbps: None,
        pot_args: None,
    }
}

/// Uma linha por argumento; `\` ⇒ `/` para o snapshot valer no Windows e no Linux.
fn render(args: &[String]) -> String {
    args.iter()
        .map(|a| a.replace('\\', "/"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn options(tmp: &str, sponsorblock: Option<Vec<&str>>) -> DownloadOptions {
    DownloadOptions {
        url: URL.to_string(),
        tmp_dir: PathBuf::from(tmp),
        sponsorblock: sponsorblock.map(|c| c.into_iter().map(String::from).collect()),
    }
}

#[test]
fn analisar_video() {
    insta::assert_snapshot!(render(&analyze_video_args(&ctx(), URL)));
}

#[test]
fn analisar_colecao() {
    let url = "https://www.youtube.com/playlist?list=OLAK5uy_x";
    insta::assert_snapshot!(render(&analyze_collection_args(&ctx(), url)));
}

#[test]
fn buscar_no_ytmusic() {
    let args = search_args(&ctx(), SearchSource::YtMusic, "rick astley & você", 5);
    insta::assert_snapshot!(render(&args));
}

#[test]
fn buscar_no_youtube() {
    let args = search_args(&ctx(), SearchSource::Youtube, "rick astley", 10);
    insta::assert_snapshot!(render(&args));
}

#[test]
fn baixar_sem_sponsorblock() {
    insta::assert_snapshot!(render(&download_args(&ctx(), &options("/tmp/job-1", None))));
}

#[test]
fn baixar_com_sponsorblock() {
    let opts = options("/tmp/job-1", Some(vec!["music_offtopic", "sponsor"]));
    insta::assert_snapshot!(render(&download_args(&ctx(), &opts)));
}

#[test]
fn sponsorblock_vazio_e_ignorado() {
    let args = download_args(&ctx(), &options("/tmp/job-1", Some(vec![])));
    assert!(!args.contains(&"--sponsorblock-remove".to_string()));
}

#[test]
fn baixar_com_cookies_de_navegador() {
    let mut ctx = ctx();
    ctx.cookies = Some(CookiesArg::Browser("firefox".to_string()));
    insta::assert_snapshot!(render(&download_args(&ctx, &options("/tmp/job-1", None))));
}

#[test]
fn baixar_com_cookies_de_arquivo() {
    let mut ctx = ctx();
    ctx.cookies = Some(CookiesArg::File(PathBuf::from("/home/u/cookies.txt")));
    insta::assert_snapshot!(render(&download_args(&ctx, &options("/tmp/job-1", None))));
}

#[test]
fn baixar_com_limite_de_velocidade() {
    let mut ctx = ctx();
    ctx.limit_rate_mbps = Some(2.5);
    insta::assert_snapshot!(render(&download_args(&ctx, &options("/tmp/job-1", None))));
}

#[test]
fn limite_zero_ou_negativo_e_ignorado() {
    for mbps in [0.0, -1.0] {
        let mut ctx = ctx();
        ctx.limit_rate_mbps = Some(mbps);
        assert!(!base_args(&ctx).contains(&"--limit-rate".to_string()));
    }
    let mut ctx = ctx();
    ctx.limit_rate_mbps = Some(2.0);
    assert!(base_args(&ctx).contains(&"2M".to_string()));
}

#[test]
fn baixar_com_pot_args() {
    let mut ctx = ctx();
    ctx.pot_args = Some(vec![
        "--plugin-dirs".to_string(),
        "/tools/bgutil/plugins".to_string(),
        "--extractor-args".to_string(),
        "youtubepot-bgutilhttp:base_url=http://127.0.0.1:4416".to_string(),
    ]);
    insta::assert_snapshot!(render(&download_args(&ctx, &options("/tmp/job-1", None))));
}

#[test]
fn caminhos_com_espacos_e_acentos_sao_um_unico_argumento() {
    let dir = r"C:\Users\Jean Carlos de Souza\Música Teste ç";
    let mut ctx = ctx();
    ctx.ffmpeg_dir = PathBuf::from(format!(r"{dir}\ferramentas\ffmpeg"));
    ctx.js_runtime_arg = format!(r"deno:{dir}\deno.exe");
    ctx.cookies = Some(CookiesArg::File(PathBuf::from(format!(
        r"{dir}\cookies.txt"
    ))));
    let opts = options(&format!(r"{dir}\tmp\job 1"), None);
    let args = download_args(&ctx, &opts);

    let normalized = |s: &str| s.replace('\\', "/");
    let dir_n = normalized(dir);
    let has = |needle: String| args.iter().any(|a| normalized(a) == needle);
    assert!(has(format!("{dir_n}/ferramentas/ffmpeg")));
    assert!(has(format!("deno:{dir_n}/deno.exe")));
    assert!(has(format!("{dir_n}/cookies.txt")));
    assert!(has(format!("{dir_n}/tmp/job 1/%(id)s.%(ext)s")));
    insta::assert_snapshot!(render(&args));
}

#[test]
fn a_url_vem_depois_do_separador() {
    let args = download_args(&ctx(), &options("/tmp/x", None));
    let n = args.len();
    assert_eq!(args[n - 2], "--");
    assert_eq!(args[n - 1], URL);
}

#[test]
fn cookies_are_present_in_every_operation() {
    for cookies in [
        CookiesArg::Browser("firefox".into()),
        CookiesArg::Browser("chrome".into()),
        CookiesArg::Browser("edge".into()),
        CookiesArg::Browser("brave".into()),
        CookiesArg::File(PathBuf::from("C:/Private cookie path/cookies.txt")),
    ] {
        let mut context = ctx();
        context.cookies = Some(cookies.clone());
        let (flag, value) = match cookies {
            CookiesArg::Browser(v) => ("--cookies-from-browser", v),
            CookiesArg::File(v) => ("--cookies", v.to_string_lossy().into_owned()),
        };
        for args in [
            analyze_video_args(&context, URL),
            analyze_collection_args(&context, URL),
            search_args(&context, SearchSource::YtMusic, "query", 5),
            download_args(&context, &options("/tmp", None)),
        ] {
            assert!(args
                .windows(2)
                .any(|pair| pair[0] == flag && pair[1] == value));
        }
    }
}
