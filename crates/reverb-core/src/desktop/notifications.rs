use crate::settings::Language;
use std::time::Duration;

#[derive(Default)]
pub struct CompletionBatch {
    started: Option<Duration>,
    titles: Vec<String>,
}
impl CompletionBatch {
    pub fn push(&mut self, now: Duration, title: String) {
        self.started.get_or_insert(now);
        self.titles.push(title);
    }
    pub fn flush(&mut self, now: Duration, language: Language) -> Vec<String> {
        if self
            .started
            .is_none_or(|start| now.saturating_sub(start) < Duration::from_secs(10))
        {
            return vec![];
        }
        self.started = None;
        let titles = std::mem::take(&mut self.titles);
        if titles.len() > 3 {
            vec![super::texts::downloaded(language, titles.len())]
        } else {
            titles
        }
    }
    pub fn clear(&mut self) {
        self.started = None;
        self.titles.clear();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test(start_paused = true)]
    async fn t3_five_completions_in_three_seconds_emit_one_notification() {
        let start = tokio::time::Instant::now();
        let mut batch = CompletionBatch::default();
        let mut fake_sink = vec![];
        for n in 0..5 {
            batch.push(start.elapsed(), format!("Track {n}"));
            tokio::time::advance(Duration::from_millis(600)).await;
        }
        fake_sink.extend(batch.flush(start.elapsed(), Language::PtBr));
        assert!(fake_sink.is_empty());
        tokio::time::advance(Duration::from_secs(7)).await;
        fake_sink.extend(batch.flush(start.elapsed(), Language::PtBr));
        assert_eq!(fake_sink, vec!["5 faixas baixadas"]);
        assert!(batch.flush(start.elapsed(), Language::PtBr).is_empty());
    }
}
