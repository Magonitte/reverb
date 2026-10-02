//! Normalização de textos para comparação (arquitetura §11.2, função `norm`).

use std::sync::LazyLock;

use regex::Regex;
use unicode_normalization::UnicodeNormalization;

/// Palavras que marcam um trecho entre `()`/`[]` como "ruído" do YouTube (sem acento: a
/// comparação é feita depois de remover os diacríticos; `NOISE_ACCENTED` cobre o texto original).
const NOISE_WORDS: &str = "official|video|vídeo|clipe|audio|áudio|lyric|lyrics|letra|legendado|\
    visualizer|hd|hq|4k|remaster|remastered|mv|oficial|explicit";

static NOISE_BRACKET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i)[(\[][^)\]]*\b(?:{NOISE_WORDS})\b[^)\]]*[)\]]"
    ))
    .expect("regex de ruído")
});

static FEAT_BRACKET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)[(\[][^)\]]*\b(?:feat|ft|featuring)\b[^)\]]*[)\]]").expect("regex de feat")
});

static FEAT_TAIL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:feat|ft|featuring)\b\.?\s.*$").expect("regex de feat final")
});

/// Remove os trechos de ruído entre `()`/`[]` (mantém marcadores de versão como "(Live)").
pub fn strip_noise_brackets(text: &str) -> String {
    NOISE_BRACKET.replace_all(text, " ").into_owned()
}

/// Remove diacríticos (NFKD + descarte das marcas combinantes).
pub fn strip_diacritics(text: &str) -> String {
    text.nfkd()
        .filter(|c| !('\u{0300}'..='\u{036f}').contains(c))
        .collect()
}

/// `norm(s)` do §11.2: sem acentos, minúsculas, sem ruído, sem "feat. …", "&" ⇒ "and", sem
/// pontuação e com espaços colapsados.
pub fn norm(text: &str) -> String {
    let lowered = strip_diacritics(text).to_lowercase();
    let without_noise = strip_noise_brackets(&lowered);
    let without_feat_brackets = FEAT_BRACKET.replace_all(&without_noise, " ");
    let without_feat = FEAT_TAIL.replace(&without_feat_brackets, "");
    let mut out = String::with_capacity(without_feat.len());
    for c in without_feat.chars() {
        match c {
            '&' => out.push_str(" and "),
            '\'' | '’' | '‘' | '`' | '"' | '“' | '”' => {}
            c if c.is_alphanumeric() => out.push(c),
            _ => out.push(' '),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Versão "leve" de `norm`: só minúsculas, sem acentos e sem pontuação. Mantém os trechos entre
/// parênteses e o "feat. …" (serve para procurar um artista dentro de um título).
pub fn norm_plain(text: &str) -> String {
    let lowered = strip_diacritics(text).to_lowercase();
    let mut out = String::with_capacity(lowered.len());
    for c in lowered.chars() {
        match c {
            '&' => out.push_str(" and "),
            '\'' | '’' | '‘' | '`' | '"' | '“' | '”' => {}
            c if c.is_alphanumeric() => out.push(c),
            _ => out.push(' '),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn norm_plain_mantem_feat_e_parenteses() {
        assert_eq!(
            norm_plain("Song (feat. Guest) [Live]"),
            "song feat guest live"
        );
    }

    #[test]
    fn normaliza_casos_do_plano() {
        let cases = [
            ("Never Gonna Give You Up", "never gonna give you up"),
            (
                "Rick Astley - Never Gonna Give You Up (Official Video) (4K Remaster)",
                "rick astley never gonna give you up",
            ),
            ("Canção do Mar", "cancao do mar"),
            ("Beyoncé", "beyonce"),
            ("Song [4K]", "song"),
            ("Song (Official Music Video)", "song"),
            ("Song (Vídeo Oficial)", "song"),
            ("Song (Lyric Video)", "song"),
            ("Song (Áudio)", "song"),
            ("Song [HD]", "song"),
            ("Song (Remastered 2009)", "song"),
            ("Song (Explicit)", "song"),
            ("Song feat. Other Artist", "song"),
            ("Song (feat. Other Artist)", "song"),
            ("Song ft. Other", "song"),
            ("Song featuring Other", "song"),
            ("Simon & Garfunkel", "simon and garfunkel"),
            ("Song (Ao Vivo)", "song ao vivo"),
            ("Song (Live)", "song live"),
            ("Song (Live at Wembley)", "song live at wembley"),
            ("\"Quoted Title\"", "quoted title"),
            ("“Aspas Curvas”", "aspas curvas"),
            ("Don't Stop Me Now", "dont stop me now"),
            ("AC/DC", "ac dc"),
            ("  Muitos    espaços  ", "muitos espacos"),
            ("Ünïcödé ÅÄÖ", "unicode aao"),
            ("", ""),
        ];
        for (input, expected) in cases {
            assert_eq!(norm(input), expected, "entrada: {input:?}");
        }
    }

    #[test]
    fn mantem_o_que_nao_e_ruido() {
        assert_eq!(norm("Song (Remix)"), "song remix");
        assert_eq!(norm("Song (Acoustic)"), "song acoustic");
        assert_eq!(norm("Song (Ao Vivo)"), "song ao vivo");
    }

    #[test]
    fn strip_noise_brackets_preserva_caixa_e_versoes() {
        assert_eq!(
            strip_noise_brackets("Title (Official Video) (Live)")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
            "Title (Live)"
        );
    }
}
