//! Análise do título bruto do YouTube em artista + título (arquitetura §11.3).
//!
//! A convenção do YouTube é **"Artista - Título"** (esquerda = artista). O parser do projeto
//! antigo invertia os lados; aqui não.

use std::sync::LazyLock;

use regex::Regex;

use super::normalize::strip_noise_brackets;

/// Separadores aceitos, na ordem de declaração; vale o que aparece primeiro no texto.
const SEPARATORS: [&str; 4] = [" - ", " – ", " — ", " | "];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTitle {
    /// Texto de exibição: `A` ou `A feat. B`. `None` quando não há como saber.
    pub artist: Option<String>,
    /// Artistas separados (principal + convidados), para a pontuação.
    pub artists: Vec<String>,
    pub title: String,
}

static FEAT_BRACKET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\s*[(\[]\s*(?:feat|ft|featuring)\b\.?\s*([^)\]]+?)\s*[)\]]")
        .expect("regex de feat entre colchetes")
});

static FEAT_TAIL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\s+(?:feat|ft|featuring)\b\.?\s+(.+)$").expect("regex de feat final")
});

static FEAT_SPLIT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\s+(?:feat|ft|featuring)\b\.?\s+").expect("regex de separação de feat")
});

static GUEST_SPLIT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\s*(?:,|&|\band\b|\be\b|\bx\b)\s*").expect("regex de convidados")
});

static CHANNEL_SUFFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:\s*-\s*topic|\s*vevo|\s+official)\s*$").expect("regex de canal")
});

static CHANNEL_PREFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^official\s+").expect("regex de prefixo de canal"));

fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Nome do canal sem " - Topic", "VEVO" e "Official".
pub fn clean_channel(channel: &str) -> Option<String> {
    let mut name = collapse(channel);
    loop {
        let next = CHANNEL_SUFFIX.replace(&name, "").trim().to_string();
        if next == name {
            break;
        }
        name = next;
    }
    let name = CHANNEL_PREFIX.replace(&name, "").trim().to_string();
    (!name.is_empty()).then_some(name)
}

fn strip_quotes(text: &str) -> String {
    let text = text.trim();
    for (open, close) in [('"', '"'), ('“', '”'), ('‘', '’'), ('«', '»'), ('\'', '\'')] {
        if text.chars().count() >= 2 && text.starts_with(open) && text.ends_with(close) {
            let inner: String = text.chars().skip(1).collect::<Vec<_>>()
                [..text.chars().count() - 2]
                .iter()
                .collect();
            // Só tira se não houver outra aspa dentro (ex.: `"A" e "B"` fica como está).
            if !inner.contains(open) && !inner.contains(close) {
                return inner.trim().to_string();
            }
        }
    }
    text.to_string()
}

/// Separa o artista principal dos convidados de um texto como `A feat. B & C`.
pub fn split_artists(display: &str) -> Vec<String> {
    let mut parts = FEAT_SPLIT.splitn(display, 2);
    let main = parts.next().unwrap_or_default().trim();
    let mut artists = Vec::new();
    if !main.is_empty() {
        artists.push(main.to_string());
    }
    if let Some(guests) = parts.next() {
        artists.extend(
            GUEST_SPLIT
                .split(guests)
                .map(str::trim)
                .filter(|g| !g.is_empty())
                .map(str::to_string),
        );
    }
    artists
}

/// Tira o "feat. X" do título e devolve (título limpo, convidados).
fn extract_feat(title: &str) -> (String, Option<String>) {
    if let Some(captures) = FEAT_BRACKET.captures(title) {
        let guests = captures[1].trim().to_string();
        let cleaned = FEAT_BRACKET.replace(title, "").into_owned();
        return (collapse(&cleaned), Some(guests));
    }
    if let Some(captures) = FEAT_TAIL.captures(title) {
        let guests = captures[1].trim().to_string();
        let cleaned = FEAT_TAIL.replace(title, "").into_owned();
        return (collapse(&cleaned), Some(guests));
    }
    (collapse(title), None)
}

fn first_separator(text: &str) -> Option<(usize, usize)> {
    SEPARATORS
        .iter()
        .filter_map(|sep| text.find(sep).map(|at| (at, sep.len())))
        .min_by_key(|(at, _)| *at)
}

pub fn parse_title(raw_title: &str, channel: Option<&str>) -> ParsedTitle {
    let raw = collapse(raw_title);
    let (left, right) = match first_separator(&raw) {
        Some((at, len)) => {
            let left = raw[..at].trim().to_string();
            let right = raw[at + len..].trim().to_string();
            if left.is_empty() || right.is_empty() {
                (None, raw.clone())
            } else {
                (Some(left), right)
            }
        }
        None => (None, raw.clone()),
    };
    let artist_side = left.map(|text| collapse(&strip_noise_brackets(&text)));
    let title_side = collapse(&strip_noise_brackets(&right));
    let (title, guests) = extract_feat(&title_side);
    let title = strip_quotes(&title);

    let base_artist = match artist_side {
        Some(artist) if !artist.is_empty() => Some(artist),
        _ => channel.and_then(clean_channel),
    };
    let artist = match (base_artist, guests) {
        (Some(artist), Some(guests)) => Some(format!("{artist} feat. {guests}")),
        (Some(artist), None) => Some(artist),
        (None, Some(guests)) => Some(guests),
        (None, None) => None,
    };
    let artists = artist.as_deref().map(split_artists).unwrap_or_default();
    ParsedTitle {
        artist,
        artists,
        title: if title.is_empty() { raw } else { title },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(raw: &str, channel: Option<&str>) -> (Option<String>, String) {
        let p = parse_title(raw, channel);
        (p.artist, p.title)
    }

    fn check(raw: &str, channel: Option<&str>, artist: Option<&str>, title: &str) {
        assert_eq!(
            parsed(raw, channel),
            (artist.map(str::to_string), title.to_string()),
            "entrada: {raw:?} / canal {channel:?}"
        );
    }

    #[test]
    fn artista_hifen_titulo() {
        check(
            "Rick Astley - Never Gonna Give You Up",
            None,
            Some("Rick Astley"),
            "Never Gonna Give You Up",
        );
        check(
            "Rick Astley - Never Gonna Give You Up (Official Video) (4K Remaster)",
            Some("Rick Astley"),
            Some("Rick Astley"),
            "Never Gonna Give You Up",
        );
        check(
            "Artista – Título (Official Video)",
            None,
            Some("Artista"),
            "Título",
        );
        check("Artista — Título [4K]", None, Some("Artista"), "Título");
        check("Artista | Título", None, Some("Artista"), "Título");
        check(
            "Artista - Título (Video Oficial)",
            None,
            Some("Artista"),
            "Título",
        );
        check(
            "Artista - Título (Lyric Video)",
            None,
            Some("Artista"),
            "Título",
        );
        check(
            "Artista - Título (Áudio Oficial)",
            None,
            Some("Artista"),
            "Título",
        );
        check("Artista - Título [HD]", None, Some("Artista"), "Título");
        check(
            "Artista - Título (Remastered 2011)",
            None,
            Some("Artista"),
            "Título",
        );
    }

    #[test]
    fn nao_inverte_os_lados() {
        let p = parse_title("Beatles - Hey Jude", None);
        assert_eq!(p.artist.as_deref(), Some("Beatles"));
        assert_eq!(p.title, "Hey Jude");
    }

    #[test]
    fn mantem_marcadores_de_versao() {
        check(
            "Artista - Título (Ao Vivo)",
            None,
            Some("Artista"),
            "Título (Ao Vivo)",
        );
        check(
            "Artista - Título (Live)",
            None,
            Some("Artista"),
            "Título (Live)",
        );
        check(
            "Artista - Título (Remix)",
            None,
            Some("Artista"),
            "Título (Remix)",
        );
        check(
            "Artista - Título (Acoustic) (Official Video)",
            None,
            Some("Artista"),
            "Título (Acoustic)",
        );
        check(
            "Artista - Título (Live at Wembley)",
            None,
            Some("Artista"),
            "Título (Live at Wembley)",
        );
    }

    #[test]
    fn sem_separador_usa_o_canal() {
        check(
            "Título",
            Some("Canal Qualquer"),
            Some("Canal Qualquer"),
            "Título",
        );
        check(
            "Título",
            Some("Rick Astley - Topic"),
            Some("Rick Astley"),
            "Título",
        );
        check(
            "Título",
            Some("RickAstleyVEVO"),
            Some("RickAstley"),
            "Título",
        );
        check(
            "Título",
            Some("Rick Astley Official"),
            Some("Rick Astley"),
            "Título",
        );
        check("Título", Some("Official Banda"), Some("Banda"), "Título");
        check("Título", None, None, "Título");
        check(
            "Título (Official Video)",
            Some("Banda"),
            Some("Banda"),
            "Título",
        );
    }

    #[test]
    fn feat_vai_para_o_artista() {
        check("A - B feat. C", None, Some("A feat. C"), "B");
        check("A - B (feat. C)", None, Some("A feat. C"), "B");
        check("A - B (ft. C)", None, Some("A feat. C"), "B");
        check("A - B [featuring C]", None, Some("A feat. C"), "B");
        check("A - B ft. C (Official Video)", None, Some("A feat. C"), "B");
        check("B feat. C", Some("A"), Some("A feat. C"), "B");
        let p = parse_title("A - B feat. C & D", None);
        assert_eq!(p.artists, vec!["A", "C", "D"]);
    }

    #[test]
    fn aspas_sao_removidas() {
        check("Artista - \"Título\"", None, Some("Artista"), "Título");
        check("Artista - “Título”", None, Some("Artista"), "Título");
        check("\"Título\"", Some("Canal"), Some("Canal"), "Título");
        check(
            "Artista - \"A\" e \"B\"",
            None,
            Some("Artista"),
            "\"A\" e \"B\"",
        );
    }

    #[test]
    fn so_o_primeiro_separador_vale() {
        check("A - B - C", None, Some("A"), "B - C");
        check("A | B - C", None, Some("A"), "B - C");
        check("A - B | C", None, Some("A"), "B | C");
    }

    #[test]
    fn hifen_sem_espacos_nao_separa() {
        check(
            "Spider-Man Theme",
            Some("Canal"),
            Some("Canal"),
            "Spider-Man Theme",
        );
        check("AC-DC", None, None, "AC-DC");
    }

    #[test]
    fn lados_vazios_nao_separam() {
        check("- Título", Some("Canal"), Some("Canal"), "- Título");
        check("Artista - ", Some("Canal"), Some("Canal"), "Artista -");
    }

    #[test]
    fn titulo_so_de_ruido_volta_ao_bruto() {
        let p = parse_title("(Official Video)", Some("Canal"));
        assert_eq!(p.title, "(Official Video)");
    }
}
