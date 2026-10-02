use crate::settings::Language;

pub fn text(language: Language, key: &str) -> &'static str {
    let english = matches!(language, Language::En);
    match (key, english) {
        ("open", false) => "Abrir",
        ("open", true) => "Open",
        ("pause", false) => "Pausar fila",
        ("pause", true) => "Pause queue",
        ("resume", false) => "Retomar fila",
        ("resume", true) => "Resume queue",
        ("clipboard", false) => "Baixar link copiado",
        ("clipboard", true) => "Download copied link",
        ("folder", false) => "Abrir pasta de músicas",
        ("folder", true) => "Open music folder",
        ("exit", false) => "Sair",
        ("exit", true) => "Quit",
        ("copied", false) => "Link copiado disponível para baixar",
        ("copied", true) => "Copied link available to download",
        ("noLink", false) => "Copie um link do YouTube para baixar",
        ("noLink", true) => "Copy a YouTube link to download",
        ("failed", false) => "Não foi possível baixar a faixa",
        ("failed", true) => "Track download failed",
        ("update", false) => "Uma atualização do Reverb está disponível",
        ("update", true) => "A Reverb update is available",
        ("heal", false) => "Reparando as ferramentas de download",
        ("heal", true) => "Repairing download tools",
        ("healFailed", false) => "A autocura precisa de atenção",
        ("healFailed", true) => "Automatic repair needs attention",
        ("diagnostics", false) => "O diagnóstico semanal encontrou problemas",
        ("diagnostics", true) => "Weekly diagnostics found problems",
        _ => "Reverb",
    }
}
pub fn active(language: Language, count: usize) -> String {
    match language {
        Language::En => format!("Reverb — {count} active"),
        _ => format!("Reverb — {count} ativos"),
    }
}
pub fn downloaded(language: Language, count: usize) -> String {
    match language {
        Language::En => format!("{count} tracks downloaded"),
        _ => format!("{count} faixas baixadas"),
    }
}
pub fn sync_summary(language: Language, title: &str, added: u32, failed: u32) -> String {
    match language {
        Language::En => format!("Playlist {title}: {added} new tracks, {failed} failed"),
        _ => format!("Playlist {title}: {added} novas faixas, {failed} falharam"),
    }
}
