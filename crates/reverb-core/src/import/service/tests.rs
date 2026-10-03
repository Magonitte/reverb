use super::*;
use crate::backend::{JobSpec, ProgressCallback};
use crate::ytdlp::{
    errors::{DownloadError, ErrorKind},
    Analysis, DoneInfo, SearchResult, SearchSource,
};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
struct CountBackend {
    active: AtomicUsize,
    peak: AtomicUsize,
}
#[async_trait::async_trait]
impl DownloadBackend for CountBackend {
    async fn analyze(&self, _: &str, _: &CancellationToken) -> Result<Analysis, DownloadError> {
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(active, Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        self.active.fetch_sub(1, Ordering::SeqCst);
        Err(DownloadError::new(ErrorKind::Unavailable, "fixture"))
    }
    async fn search(
        &self,
        _: SearchSource,
        _: &str,
        _: u32,
        _: &CancellationToken,
    ) -> Result<Vec<SearchResult>, DownloadError> {
        Ok(Vec::new())
    }
    async fn download(
        &self,
        _: &JobSpec,
        _: ProgressCallback<'_>,
        _: &CancellationToken,
    ) -> Result<DoneInfo, DownloadError> {
        unreachable!()
    }
}
#[tokio::test]
async fn candidate_analyses_share_three_slots_and_cancel_waiters() {
    let inner = Arc::new(CountBackend::default());
    let backend = BoundedBackend {
        inner: inner.clone(),
        slots: tokio::sync::Semaphore::new(3),
    };
    let cancel = CancellationToken::new();
    futures_util::future::join_all((0..12).map(|_| backend.analyze("fixture", &cancel))).await;
    assert_eq!(inner.peak.load(Ordering::SeqCst), 3);
    assert_eq!(inner.active.load(Ordering::SeqCst), 0);
    let slots = backend.slots.acquire_many(3).await.unwrap();
    cancel.cancel();
    assert_eq!(
        backend.analyze("fixture", &cancel).await.unwrap_err().kind,
        ErrorKind::Cancelled
    );
    drop(slots);
}
