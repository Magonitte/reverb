use crate::settings::SplitChapters;
use crate::ytdlp::VideoInfo;
use regex::Regex;
use std::sync::LazyLock;

static PREFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:(?:\d{1,2}:){1,2}\d{2}(?:\.\d+)?\s*[-–—.]?\s*|\d{1,3}\s*[.\-)–—]\s*)")
        .expect("chapter prefix")
});
pub fn clean_title(title: &str) -> String {
    let mut result = title.trim().to_string();
    loop {
        let cleaned = PREFIX.replace(&result, "").trim().to_string();
        if cleaned == result {
            return result;
        }
        result = cleaned;
    }
}
pub fn should_split(video: &VideoInfo, setting: SplitChapters, choice: Option<bool>) -> bool {
    video.chapters.len() >= 2
        && video.duration.is_some_and(|d| d > 600.0)
        && choice.unwrap_or(setting == SplitChapters::Always)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn titles() {
        for (input, want) in [
            ("01. Intro", "Intro"),
            ("1 - Song", "Song"),
            ("00:00 Intro", "Intro"),
            ("1:02:03 Song", "Song"),
            ("02) Next", "Next"),
            (" 03. 00:10 Finale ", "Finale"),
            ("Song 2", "Song 2"),
            ("1987", "1987"),
            ("01 - 東京", "東京"),
        ] {
            assert_eq!(clean_title(input), want);
        }
    }
}
