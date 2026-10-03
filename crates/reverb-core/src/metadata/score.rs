//! Pontuação de correspondência entre a faixa-base e um candidato (arquitetura §11.2).

use std::sync::LazyLock;

use regex::Regex;

use super::normalize::norm;

/// O que se compara: título, artistas e (se houver) duração em segundos.
#[derive(Debug, Clone, PartialEq)]
pub struct Subject {
    pub title: String,
    pub artists: Vec<String>,
    pub duration_s: Option<f64>,
}

impl Subject {
    pub fn new(title: &str, artists: &[&str], duration_s: Option<f64>) -> Self {
        Self {
            title: title.to_string(),
            artists: artists.iter().map(|a| (*a).to_string()).collect(),
            duration_s,
        }
    }
}

/// Tolerância de duração: `Normal` para faixas, `Clip` quando a fonte é um clipe (não oficial).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurationTolerance {
    Normal,
    Clip,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoreDetail {
    pub title_sim: f64,
    pub artist_sim: f64,
    /// `None` quando algum dos lados não tem duração.
    pub dur_score: Option<f64>,
    pub version_penalty: bool,
    pub score: f64,
}

/// Tetos do §11.2.
pub const NO_DURATION_CAP: f64 = 0.84;
const VERSION_FACTOR: f64 = 0.6;

static VERSION_MARKER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\b(?:live|ao vivo|remix|acoustic|acustico|instrumental|karaoke|cover|sped up|slowed|nightcore|8d)\b",
    )
    .expect("regex de versão")
});

/// O texto (já normalizado) contém um marcador de versão do §11.2?
pub fn has_version_marker(normalized: &str) -> bool {
    VERSION_MARKER.is_match(normalized)
}

/// `a` aparece em `b` como sequência inteira de palavras?
fn contains_words(haystack: &str, needle: &str) -> bool {
    format!(" {haystack} ").contains(&format!(" {needle} "))
}

/// Similaridade de duas strings já normalizadas.
fn sim_normalized(a: &str, b: &str) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let base = strsim::jaro_winkler(a, b);
    let long_enough = a.chars().count() >= 3 && b.chars().count() >= 3;
    if long_enough && (contains_words(a, b) || contains_words(b, a)) {
        base.max(0.95)
    } else {
        base
    }
}

/// `sim(a, b)` do §11.2 (Jaro-Winkler sobre `norm`, com piso 0,95 para contenção).
pub fn sim(a: &str, b: &str) -> f64 {
    sim_normalized(&norm(a), &norm(b))
}

/// Maior `sim` entre todos os pares de artistas.
pub fn artist_sim(base: &[String], candidate: &[String]) -> f64 {
    let normalized: Vec<String> = candidate.iter().map(|a| norm(a)).collect();
    base.iter()
        .map(|a| norm(a))
        .flat_map(|a| {
            normalized
                .iter()
                .map(move |c| sim_normalized(&a, c))
                .collect::<Vec<_>>()
        })
        .fold(0.0, f64::max)
}

/// Pontuação de duração: tolerância normal Δ ≤ 2 s ⇒ 1 e Δ ≥ 10 s ⇒ 0; clipe Δ ≤ 5 ⇒ 1 e
/// Δ ≥ 45 ⇒ 0; linear no meio.
pub fn dur_score(delta_s: f64, tolerance: DurationTolerance) -> f64 {
    let (full, zero) = match tolerance {
        DurationTolerance::Normal => (2.0, 10.0),
        DurationTolerance::Clip => (5.0, 45.0),
    };
    let delta = delta_s.abs();
    if delta <= full {
        1.0
    } else if delta >= zero {
        0.0
    } else {
        (zero - delta) / (zero - full)
    }
}

/// Exatamente um dos títulos tem marcador de versão (live, remix…)?
pub fn version_mismatch(base_title: &str, candidate_title: &str) -> bool {
    has_version_marker(&norm(base_title)) != has_version_marker(&norm(candidate_title))
}

/// A fórmula do §11.2 a partir das partes já calculadas.
pub fn combine(
    title_sim: f64,
    artist_sim: f64,
    dur_score: Option<f64>,
    version_penalty: bool,
) -> f64 {
    let mut score = match dur_score {
        Some(dur) => 0.45 * title_sim + 0.35 * artist_sim + 0.20 * dur,
        None => (0.55 * title_sim + 0.45 * artist_sim).min(NO_DURATION_CAP),
    };
    if version_penalty {
        score *= VERSION_FACTOR;
    }
    score
}

/// Diferença de duração em pontuação (`None` se algum lado não tem duração).
pub fn duration_part(
    base: Option<f64>,
    candidate: Option<f64>,
    tolerance: DurationTolerance,
) -> Option<f64> {
    match (base, candidate) {
        (Some(a), Some(b)) => Some(dur_score(a - b, tolerance)),
        _ => None,
    }
}

/// Pontuação completa com o detalhamento de cada parte.
pub fn score_detail(
    base: &Subject,
    candidate: &Subject,
    tolerance: DurationTolerance,
) -> ScoreDetail {
    let title_sim = sim(&base.title, &candidate.title);
    let artist_sim = artist_sim(&base.artists, &candidate.artists);
    let dur = duration_part(base.duration_s, candidate.duration_s, tolerance);
    let version_penalty = version_mismatch(&base.title, &candidate.title);
    let score = combine(title_sim, artist_sim, dur, version_penalty);
    ScoreDetail {
        title_sim,
        artist_sim,
        dur_score: dur,
        version_penalty,
        score,
    }
}

pub fn score(base: &Subject, candidate: &Subject, tolerance: DurationTolerance) -> f64 {
    score_detail(base, candidate, tolerance).score
}

#[cfg(test)]
mod tests {
    use super::*;
    use DurationTolerance::{Clip, Normal};

    struct Case {
        name: &'static str,
        base: Subject,
        candidate: Subject,
        tolerance: DurationTolerance,
        min: f64,
        max: f64,
    }

    fn case(
        name: &'static str,
        base: Subject,
        candidate: Subject,
        tolerance: DurationTolerance,
        min: f64,
        max: f64,
    ) -> Case {
        Case {
            name,
            base,
            candidate,
            tolerance,
            min,
            max,
        }
    }

    fn cases() -> Vec<Case> {
        let rick = |t: &str, d: Option<f64>| Subject::new(t, &["Rick Astley"], d);
        vec![
            case(
                "idêntico",
                rick("Never Gonna Give You Up", Some(214.0)),
                rick("Never Gonna Give You Up", Some(214.0)),
                Normal,
                0.95,
                1.0,
            ),
            case(
                "caixa e acento diferentes",
                Subject::new("Canção do Mar", &["Amália"], Some(200.0)),
                Subject::new("cancao do mar", &["amalia"], Some(200.0)),
                Normal,
                0.95,
                1.0,
            ),
            case(
                "Δ 2 s ainda é pleno",
                rick("Never Gonna Give You Up", Some(214.0)),
                rick("Never Gonna Give You Up", Some(216.0)),
                Normal,
                0.95,
                1.0,
            ),
            case(
                "Δ 6 s perde um pouco",
                rick("Never Gonna Give You Up", Some(214.0)),
                rick("Never Gonna Give You Up", Some(220.0)),
                Normal,
                0.88,
                0.97,
            ),
            case(
                "Δ 12 s zera a duração",
                rick("Never Gonna Give You Up", Some(214.0)),
                rick("Never Gonna Give You Up", Some(226.0)),
                Normal,
                0.75,
                0.82,
            ),
            case(
                "mesmo título, artista diferente",
                rick("Never Gonna Give You Up", Some(214.0)),
                Subject::new("Never Gonna Give You Up", &["Someone Else"], Some(214.0)),
                Normal,
                0.0,
                0.849,
            ),
            case(
                "título diferente, mesmo artista",
                rick("Never Gonna Give You Up", Some(214.0)),
                rick("Together Forever", Some(214.0)),
                Normal,
                0.0,
                0.849,
            ),
            case(
                "Song × Song (Live) penalizado",
                Subject::new("Song", &["Band"], Some(200.0)),
                Subject::new("Song (Live)", &["Band"], Some(200.0)),
                Normal,
                0.0,
                0.7,
            ),
            case(
                "Song (Live) × Song (Live) sem penalidade",
                Subject::new("Song (Live)", &["Band"], Some(200.0)),
                Subject::new("Song (Live)", &["Band"], Some(200.0)),
                Normal,
                0.95,
                1.0,
            ),
            case(
                "remix só de um lado",
                Subject::new("Song", &["Band"], Some(200.0)),
                Subject::new("Song (Remix)", &["Band"], Some(200.0)),
                Normal,
                0.0,
                0.7,
            ),
            case(
                "Ao Vivo só de um lado",
                Subject::new("Canção (Ao Vivo)", &["Banda"], Some(200.0)),
                Subject::new("Canção", &["Banda"], Some(200.0)),
                Normal,
                0.0,
                0.7,
            ),
            case(
                "instrumental só de um lado",
                Subject::new("Song", &["Band"], Some(200.0)),
                Subject::new("Song Instrumental", &["Band"], Some(200.0)),
                Normal,
                0.0,
                0.7,
            ),
            case(
                "sem duração no candidato ⇒ ≤ 0,84",
                rick("Never Gonna Give You Up", Some(214.0)),
                rick("Never Gonna Give You Up", None),
                Normal,
                0.80,
                0.84,
            ),
            case(
                "sem duração na base ⇒ ≤ 0,84",
                rick("Never Gonna Give You Up", None),
                rick("Never Gonna Give You Up", Some(214.0)),
                Normal,
                0.80,
                0.84,
            ),
            case(
                "sem duração nos dois ⇒ ≤ 0,84",
                rick("Never Gonna Give You Up", None),
                rick("Never Gonna Give You Up", None),
                Normal,
                0.80,
                0.84,
            ),
            case(
                "clipe Δ 1 s",
                rick("Never Gonna Give You Up", Some(213.0)),
                rick("Never Gonna Give You Up", Some(214.0)),
                Clip,
                0.95,
                1.0,
            ),
            case(
                "clipe Δ 30 s tem dur_score > 0",
                rick("Never Gonna Give You Up", Some(184.0)),
                rick("Never Gonna Give You Up", Some(214.0)),
                Clip,
                0.87,
                0.88,
            ),
            case(
                "Δ 30 s na tolerância normal zera",
                rick("Never Gonna Give You Up", Some(184.0)),
                rick("Never Gonna Give You Up", Some(214.0)),
                Normal,
                0.75,
                0.82,
            ),
            case(
                "clipe Δ 50 s zera",
                rick("Never Gonna Give You Up", Some(164.0)),
                rick("Never Gonna Give You Up", Some(214.0)),
                Clip,
                0.75,
                0.82,
            ),
            case(
                "(Official Video) é ignorado no título",
                rick(
                    "Never Gonna Give You Up (Official Video) (4K Remaster)",
                    Some(213.0),
                ),
                rick("Never Gonna Give You Up", Some(214.0)),
                Clip,
                0.95,
                1.0,
            ),
            case(
                "feat. é ignorado no título",
                Subject::new("Song feat. Guest", &["Band"], Some(200.0)),
                Subject::new("Song", &["Band"], Some(200.0)),
                Normal,
                0.95,
                1.0,
            ),
            case(
                "vários artistas: basta um par",
                Subject::new("Song", &["A", "Band"], Some(200.0)),
                Subject::new("Song", &["Band"], Some(200.0)),
                Normal,
                0.95,
                1.0,
            ),
            case(
                "totalmente diferente",
                Subject::new("Aaaa", &["Bbbb"], Some(100.0)),
                Subject::new("Zzzz Yyyy", &["Xxxx"], Some(300.0)),
                Normal,
                0.0,
                0.5,
            ),
            case(
                "erro real do app antigo: Me at the zoo × At the Zoo",
                Subject::new("Me at the zoo", &["jawed"], Some(19.0)),
                Subject::new("At the Zoo", &["Deborah Lurie"], Some(120.0)),
                Normal,
                0.0,
                0.849,
            ),
            case(
                "mesmo caso, sem duração do candidato",
                Subject::new("Me at the zoo", &["jawed"], Some(19.0)),
                Subject::new("At the Zoo", &["Deborah Lurie"], None),
                Normal,
                0.0,
                0.849,
            ),
        ]
    }

    #[test]
    fn tabela_de_pares_fica_na_faixa_esperada() {
        let all = cases();
        assert!(all.len() >= 20);
        for c in all {
            let value = score(&c.base, &c.candidate, c.tolerance);
            assert!(
                (c.min..=c.max).contains(&value),
                "{}: {value:.4} fora de {}..={}",
                c.name,
                c.min,
                c.max
            );
        }
    }

    #[test]
    fn sem_duracao_nunca_passa_de_084() {
        let a = Subject::new("Same", &["Same"], None);
        assert!(score(&a, &a.clone(), Normal) <= NO_DURATION_CAP + 1e-12);
    }

    #[test]
    fn clipe_com_30_s_tem_dur_score_positivo() {
        assert!(dur_score(30.0, Clip) > 0.0);
        assert_eq!(dur_score(30.0, Normal), 0.0);
        assert_eq!(dur_score(2.0, Normal), 1.0);
        assert_eq!(dur_score(10.0, Normal), 0.0);
        assert!((dur_score(6.0, Normal) - 0.5).abs() < 1e-9);
        assert_eq!(dur_score(5.0, Clip), 1.0);
        assert_eq!(dur_score(45.0, Clip), 0.0);
    }

    #[test]
    fn contencao_usa_palavras_inteiras() {
        assert!(sim("Song", "Song Live") >= 0.95);
        assert!(sim("Ice", "Police") < 0.95);
        assert!(sim("ab", "abc") < 0.95);
    }
}
