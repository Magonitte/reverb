//! F08 — T10 e T10b: versão oficial (E1) com as respostas gravadas do yt-dlp.

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use super::*;
use crate::queue::fake::FakeBackend;
use crate::ytdlp::{SearchResult, SearchSource, VideoInfo};
use DurationTolerance::{Clip, Normal};

fn fixture_text(name: &str) -> String {
    let path = format!(
        "{}/../../tests/fixtures/ytdlp/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path).unwrap()
}

fn video(name: &str) -> VideoInfo {
    VideoInfo::from_json(&fixture_text(name)).unwrap()
}

fn search(name: &str) -> Vec<SearchResult> {
    SearchResult::list_from_json(&fixture_text(name)).unwrap()
}

fn s(title: &str, artists: &[&str], duration: Option<f64>) -> Subject {
    Subject::new(title, artists, duration)
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|i| (*i).to_string()).collect()
}

// ---------------------------------------------------------------------------------------------
// T10b — funções puras
// ---------------------------------------------------------------------------------------------

#[test]
fn is_valid_isrc_aceita_e_rejeita() {
    let valid = [
        "GBARL9300135",
        "GBARL0600786",
        "USRC17607839",
        "BRBMG1900001",
        "FR6V80000001",
    ];
    for isrc in valid {
        assert!(is_valid_isrc(isrc), "{isrc} deveria ser válido");
    }
    let invalid = [
        "",
        "GBARL930013",     // 11 caracteres
        "GBARL93001350",   // 13 caracteres
        "gbarl9300135",    // minúsculas
        "GB-ARL-93-00135", // com hífens
        "1BARL9300135",    // país com dígito
        "GBARL93O0135",    // letra no lugar de dígito
        " GBARL9300135",   // espaço
        "GBARL9300135\n",
    ];
    for isrc in invalid {
        assert!(!is_valid_isrc(isrc), "{isrc:?} deveria ser inválido");
    }
}

#[test]
fn normalize_isrc_limpa_e_valida() {
    assert_eq!(
        normalize_isrc("gb-arl-93-00135").as_deref(),
        Some("GBARL9300135")
    );
    assert_eq!(
        normalize_isrc("GB ARL 93 00135").as_deref(),
        Some("GBARL9300135")
    );
    assert_eq!(
        normalize_isrc("GBARL9300135").as_deref(),
        Some("GBARL9300135")
    );
    assert_eq!(normalize_isrc("lixo"), None);
}

#[test]
fn filtros_eliminatorios() {
    struct Row {
        name: &'static str,
        query: Subject,
        result: Subject,
        tolerance: DurationTolerance,
        via_isrc: bool,
        expected: Option<Elimination>,
    }
    let row = |name, query, result, tolerance, via_isrc, expected| Row {
        name,
        query,
        result,
        tolerance,
        via_isrc,
        expected,
    };
    let rick = |title: &str, d: Option<f64>| s(title, &["Rick Astley"], d);
    let rows = vec![
        row(
            "idêntico",
            rick("Never Gonna Give You Up", Some(213.0)),
            rick("Never Gonna Give You Up", Some(214.0)),
            Clip,
            false,
            None,
        ),
        row(
            "caixa e acentos",
            s("Canção do Mar", &["Amália"], Some(200.0)),
            s("cancao do mar", &["AMALIA"], Some(200.0)),
            Normal,
            false,
            None,
        ),
        row(
            "nenhuma palavra em comum",
            rick("Never Gonna Give You Up", None),
            rick("Together Forever", None),
            Normal,
            false,
            Some(Elimination::NoCommonWord),
        ),
        row(
            "palavra em comum mas título diferente",
            s("Love Story", &["Taylor Swift"], None),
            s("Story Of My Life", &["Taylor Swift"], None),
            Normal,
            false,
            Some(Elimination::TitleSimilarity),
        ),
        row(
            "artista diferente",
            rick("Never Gonna Give You Up", None),
            s("Never Gonna Give You Up", &["Deborah Lurie"], None),
            Normal,
            false,
            Some(Elimination::ArtistSimilarity),
        ),
        row(
            "artista diferente é aceito pelo ISRC",
            rick("Never Gonna Give You Up", None),
            s("Never Gonna Give You Up", &["Deborah Lurie"], None),
            Normal,
            true,
            None,
        ),
        row(
            "Δ 20 s em faixa",
            rick("Never Gonna Give You Up", Some(214.0)),
            rick("Never Gonna Give You Up", Some(234.0)),
            Normal,
            false,
            Some(Elimination::Duration),
        ),
        row(
            "Δ 15 s em faixa ainda passa",
            rick("Never Gonna Give You Up", Some(214.0)),
            rick("Never Gonna Give You Up", Some(229.0)),
            Normal,
            false,
            None,
        ),
        row(
            "Δ 20 s em clipe passa",
            rick("Never Gonna Give You Up", Some(214.0)),
            rick("Never Gonna Give You Up", Some(234.0)),
            Clip,
            false,
            None,
        ),
        row(
            "Δ 50 s em clipe cai",
            rick("Never Gonna Give You Up", Some(214.0)),
            rick("Never Gonna Give You Up", Some(264.0)),
            Clip,
            false,
            Some(Elimination::Duration),
        ),
        row(
            "duração só de um lado não elimina",
            rick("Never Gonna Give You Up", Some(214.0)),
            rick("Never Gonna Give You Up", None),
            Normal,
            false,
            None,
        ),
        row(
            "Δ grande vale também no ISRC",
            rick("Never Gonna Give You Up", Some(214.0)),
            rick("Never Gonna Give You Up", Some(400.0)),
            Normal,
            true,
            Some(Elimination::Duration),
        ),
        row(
            "palavras de versão acumuladas derrubam o título",
            s("Song", &["Band"], None),
            s("Song Live Remix Acoustic Instrumental", &["Band"], None),
            Normal,
            false,
            Some(Elimination::TitleSimilarity),
        ),
        row(
            "uma palavra de versão ainda passa",
            s("Song", &["Band"], None),
            s("Song (Live)", &["Band"], None),
            Normal,
            false,
            None,
        ),
        row(
            "palavra de versão presente nos dois lados não penaliza",
            s("Song (Live)", &["Band"], None),
            s("Song (Live)", &["Band"], None),
            Normal,
            false,
            None,
        ),
        row(
            "dois artistas, o resultado tem um só",
            s("Song", &["A", "B"], None),
            s("Song", &["A"], None),
            Normal,
            false,
            Some(Elimination::ArtistSimilarity),
        ),
        row(
            "dois artistas, o outro está no título",
            s("Song", &["A", "B"], None),
            s("Song (feat. B)", &["A"], None),
            Normal,
            false,
            None,
        ),
        row(
            "resultado sem artista",
            rick("Never Gonna Give You Up", None),
            s("Never Gonna Give You Up", &[], None),
            Normal,
            false,
            Some(Elimination::ArtistSimilarity),
        ),
    ];
    assert!(rows.len() >= 12);
    for r in rows {
        assert_eq!(
            eliminatory_filters(&r.query, &r.result, r.tolerance, r.via_isrc),
            r.expected,
            "{}",
            r.name
        );
    }
}

#[test]
fn penalidade_de_versao_e_cumulativa() {
    assert_eq!(version_word_penalty("Song", "Song"), 0.0);
    assert!((version_word_penalty("Song", "Song (Live)") - 0.15).abs() < 1e-9);
    assert!((version_word_penalty("Song", "Song (Live) (Remix)") - 0.30).abs() < 1e-9);
    assert!((version_word_penalty("Song", "Song Live Remix Acoustic") - 0.45).abs() < 1e-9);
    // Presente também na busca ⇒ sem penalidade.
    assert_eq!(version_word_penalty("Song (Live)", "Song (Live)"), 0.0);
    assert!((version_word_penalty("Song (Live)", "Song (Live) (Remix)") - 0.15).abs() < 1e-9);
    // Palavras entre colchetes e acentuadas também contam.
    assert!((version_word_penalty("Canção", "Canção (Ao Vivo)") - 0.15).abs() < 1e-9);
    assert!((version_word_penalty("Canção", "Canção [Acústico]") - 0.15).abs() < 1e-9);
    assert!((version_word_penalty("Song", "Song (2022 Remastered)") - 0.15).abs() < 1e-9);
    assert!((version_word_penalty("Song", "Song (Bass Boosted)") - 0.15).abs() < 1e-9);
    assert!((version_word_penalty("Song", "Song (Sped Up)") - 0.15).abs() < 1e-9);
    assert!((version_word_penalty("Song", "Song - Live at the Concert") - 0.30).abs() < 1e-9);
}

#[test]
fn resultado_com_live_e_remix_perde_030_no_title_sim() {
    let query = s("Song", &["Band"], None);
    let plain = s("Song", &["Band"], None);
    let both = s("Song (Live) (Remix)", &["Band"], None);
    let (plain_sim, _) = match_parts(&query, &plain);
    let (both_sim, _) = match_parts(&query, &both);
    // "Song" está contido em "Song live remix" ⇒ sim 0,95; 0,95 − 0,30 = 0,65.
    assert!((plain_sim - 1.0).abs() < 1e-9);
    assert!((both_sim - 0.65).abs() < 1e-9, "{both_sim}");
}

#[test]
fn artistas_multiplos() {
    let a = strings(&["A"]);
    let ab = strings(&["A", "B"]);
    let abc = strings(&["A", "B", "C"]);
    // Todos presentes.
    assert!((multi_artist_sim(&ab, &ab, "Song") - 1.0).abs() < 1e-9);
    // Ordem diferente.
    assert!((multi_artist_sim(&ab, &strings(&["B", "A"]), "Song") - 1.0).abs() < 1e-9);
    // Falta um de dois.
    assert!((multi_artist_sim(&ab, &a, "Song") - 0.5).abs() < 1e-9);
    // Falta um de três.
    assert!((multi_artist_sim(&abc, &ab, "Song") - 2.0 / 3.0).abs() < 1e-9);
    // O outro artista aparece no título do resultado.
    assert!((multi_artist_sim(&ab, &a, "Song (feat. B)") - 1.0).abs() < 1e-9);
    // Um único nome composto cobre todos.
    assert!(
        (multi_artist_sim(
            &strings(&["Simon", "Garfunkel"]),
            &strings(&["Simon & Garfunkel"]),
            "Song"
        ) - 1.0)
            .abs()
            < 1e-9
    );
    assert!((multi_artist_sim(&abc, &strings(&["A, B & C"]), "Song") - 1.0).abs() < 1e-9);
    // Nenhum encontrado: cai para a maior similaridade de par (< 0,85).
    let none = multi_artist_sim(
        &strings(&["Rick Astley"]),
        &strings(&["Deborah Lurie"]),
        "Song",
    );
    assert!(none < 0.70, "{none}");
    // Sem artistas de um dos lados.
    assert_eq!(multi_artist_sim(&[], &a, "Song"), 0.0);
    assert_eq!(multi_artist_sim(&a, &[], "Song"), 0.0);
    // Semelhança com erro de digitação ainda conta.
    assert!(
        (multi_artist_sim(
            &strings(&["Rick Astley"]),
            &strings(&["Rick Astly"]),
            "Song"
        ) - 1.0)
            .abs()
            < 1e-9
    );
}

#[test]
fn desempate_prefere_a_faixa_oficial_ate_008() {
    let video = Scored {
        score: 0.95,
        official: false,
    };
    let track = Scored {
        score: 0.90,
        official: true,
    };
    // Oficial a 0,05 do vídeo ⇒ oficial vence.
    assert_eq!(tie_break(&[video, track]), Some(1));
    // Exatamente 0,08 de diferença ainda vale.
    let video = Scored {
        score: 0.98,
        official: false,
    };
    let track = Scored {
        score: 0.90,
        official: true,
    };
    assert_eq!(tie_break(&[video, track]), Some(1));
    // Mais de 0,08 ⇒ vence a maior pontuação.
    let video = Scored {
        score: 0.99,
        official: false,
    };
    let track = Scored {
        score: 0.90,
        official: true,
    };
    assert_eq!(tie_break(&[video, track]), Some(0));
    // Dois oficiais ⇒ o de maior pontuação.
    let a = Scored {
        score: 0.88,
        official: true,
    };
    let b = Scored {
        score: 0.92,
        official: true,
    };
    assert_eq!(tie_break(&[a, b]), Some(1));
    // Dois não oficiais ⇒ o de maior pontuação.
    let a = Scored {
        score: 0.90,
        official: false,
    };
    let b = Scored {
        score: 0.85,
        official: false,
    };
    assert_eq!(tie_break(&[a, b]), Some(0));
    assert_eq!(tie_break(&[]), None);
}

// ---------------------------------------------------------------------------------------------
// T10 — busca com as respostas gravadas
// ---------------------------------------------------------------------------------------------

fn backend() -> Arc<FakeBackend> {
    let backend = FakeBackend::new();
    for name in [
        "fx1-video.json",
        "fx2-music.json",
        "fx3-clip.json",
        "analyze-aUajNfZwkjY.json",
        "analyze-rmQuHi7a8Q4.json",
        "analyze-QonqLGRyMLk.json",
        "analyze-0nrIroJpjgg.json",
        "analyze--aIiQj79b6Q.json",
        "analyze-wHoUcB87euE.json",
        "analyze-OLb0YQJfos0.json",
    ] {
        backend.set_video(video(name));
    }
    backend.set_search(
        SearchSource::YtMusic,
        "Rick Astley Never Gonna Give You Up",
        search("search-ytmusic-fx3.json"),
    );
    backend.set_search(
        SearchSource::YtMusic,
        "GBARL9300135",
        search("search-ytmusic-isrc-GBARL9300135.json"),
    );
    backend.set_search(
        SearchSource::YtMusic,
        "GBARL0600786",
        search("search-ytmusic-isrc-GBARL0600786.json"),
    );
    backend
}

#[tokio::test]
async fn t10_clipe_do_fx3_encontra_a_faixa_oficial_por_texto() {
    let backend = backend();
    let clip = video("fx3-clip.json");
    let cancel = CancellationToken::new();
    let found = find_official(backend.as_ref(), &clip, None, &cancel)
        .await
        .expect("deveria achar a versão oficial");
    assert_eq!(found.matched.video_id, "lYBUbBu4W08");
    assert_eq!(
        found.matched.url,
        "https://music.youtube.com/watch?v=lYBUbBu4W08"
    );
    assert!(found.matched.score >= MIN_SCORE, "{}", found.matched.score);
    assert_eq!(found.matched.via, OfficialVia::Text);
    assert_eq!(found.matched.title, "Never Gonna Give You Up");
    assert_eq!(found.matched.artist, "Rick Astley");
    assert_eq!(
        found.matched.album.as_deref(),
        Some("Whenever You Need Somebody")
    );
    assert_eq!(found.info.id, "lYBUbBu4W08");
    // Os 3 primeiros foram analisados.
    assert_eq!(backend.analyze_log().len(), 3);
}

#[tokio::test]
async fn t10_video_sem_correspondencia_devolve_none() {
    let backend = backend();
    let zoo = video("fx1-video.json");
    let cancel = CancellationToken::new();
    assert!(find_official(backend.as_ref(), &zoo, None, &cancel)
        .await
        .is_none());
}

#[tokio::test]
async fn t10_resultados_nao_relacionados_sao_eliminados() {
    // A busca devolve só as duas faixas que não têm nada a ver com o clipe.
    let backend = backend();
    let all = search("search-ytmusic-fx3.json");
    backend.set_search(
        SearchSource::YtMusic,
        "Rick Astley Never Gonna Give You Up",
        all.into_iter().skip(1).collect(),
    );
    let clip = video("fx3-clip.json");
    let cancel = CancellationToken::new();
    assert!(find_official(backend.as_ref(), &clip, None, &cancel)
        .await
        .is_none());
}

#[tokio::test]
async fn faixa_ja_oficial_nao_busca_nada() {
    let backend = backend();
    let official = video("fx2-music.json");
    let cancel = CancellationToken::new();
    assert!(find_official(backend.as_ref(), &official, None, &cancel)
        .await
        .is_none());
    assert!(backend.search_log().is_empty());
}

#[tokio::test]
async fn isrc_aceita_a_faixa_certa_mesmo_com_varios_oficiais() {
    // GBARL9300135 devolve lYBUbBu4W08 e duas faixas sem relação (uma delas oficial).
    let backend = backend();
    let clip = video("fx3-clip.json");
    let cancel = CancellationToken::new();
    let found = find_official(backend.as_ref(), &clip, Some("GBARL9300135"), &cancel)
        .await
        .unwrap();
    assert_eq!(found.matched.video_id, "lYBUbBu4W08");
    assert_eq!(found.matched.via, OfficialVia::Isrc);
    assert_eq!(found.matched.isrc.as_deref(), Some("GBARL9300135"));
    // Nem chegou a buscar por texto.
    assert_eq!(
        backend.search_log(),
        vec!["ytmusic:GBARL9300135".to_string()]
    );
}

#[tokio::test]
async fn isrc_da_coletanea_devolve_outra_faixa_oficial() {
    let backend = backend();
    let clip = video("fx3-clip.json");
    let cancel = CancellationToken::new();
    let found = find_official(backend.as_ref(), &clip, Some("GBARL0600786"), &cancel)
        .await
        .unwrap();
    assert_eq!(found.matched.video_id, "-aIiQj79b6Q");
    assert_ne!(found.matched.video_id, "lYBUbBu4W08");
    assert_eq!(
        found.matched.album.as_deref(),
        Some("Reeling In The Decades")
    );
}

#[tokio::test]
async fn isrc_invalido_cai_para_o_texto() {
    let backend = backend();
    let clip = video("fx3-clip.json");
    let cancel = CancellationToken::new();
    let found = find_official(backend.as_ref(), &clip, Some("lixo"), &cancel)
        .await
        .unwrap();
    assert_eq!(found.matched.video_id, "lYBUbBu4W08");
    assert_eq!(found.matched.via, OfficialVia::Text);
    assert!(!backend.search_log().iter().any(|q| q.contains("lixo")));
}

#[tokio::test]
async fn isrc_sem_resultado_cai_para_o_texto() {
    let backend = backend();
    let clip = video("fx3-clip.json");
    let cancel = CancellationToken::new();
    let found = find_official(backend.as_ref(), &clip, Some("ZZZZZ0000000"), &cancel)
        .await
        .unwrap();
    assert_eq!(found.matched.video_id, "lYBUbBu4W08");
    assert_eq!(found.matched.via, OfficialVia::Text);
    assert_eq!(backend.search_log().len(), 2);
}
