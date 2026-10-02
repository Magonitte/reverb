use super::*;

#[tokio::test]
#[ignore = "network"]
async fn f13_t9_firefox_cookies_are_tested_or_browser_missing_is_reported() {
    let stack = stack().await;
    stack
        .settings
        .update(serde_json::from_value(serde_json::json!({"cookiesSource":"firefox"})).unwrap())
        .await
        .unwrap();
    let result =
        reverb_core::quality::test_cookies(stack.backend.as_ref(), &CancellationToken::new()).await;
    if !result.ok {
        assert_eq!(result.error_kind.as_deref(), Some("browser_not_found"));
    } else {
        assert!(result.best_audio.is_some());
    }
}

#[tokio::test]
#[ignore = "network"]
async fn f13_t10_real_album_chapters_create_six_organized_tracks() {
    let stack = stack().await;
    stack.settings.update(serde_json::from_value(serde_json::json!({"preferOfficialAudio":false,"splitChapters":"always","fetchLyrics":false})).unwrap()).await.unwrap();
    let job = run_job(
        &stack,
        "https://www.youtube.com/watch?v=OZTNn4wlegM",
        "OZTNn4wlegM",
    )
    .await;
    assert!(job.library_id.is_some());
    let page = stack
        .db
        .call(|conn| reverb_core::library::list(conn, &Default::default()))
        .await
        .unwrap();
    assert_eq!(page.total, 6);
    let album = page.items[0].album.clone().unwrap();
    let mut numbers = Vec::new();
    for item in page.items {
        assert_eq!(item.album.as_deref(), Some(album.as_str()));
        assert_eq!(item.track_total, Some(6));
        numbers.push(item.track_no.unwrap());
        assert!(std::path::Path::new(&item.file_path).is_file());
        let tags = reverb_core::tagging::read_tags(std::path::Path::new(&item.file_path)).unwrap();
        assert_eq!(tags.track_total, Some(6));
    }
    numbers.sort();
    assert_eq!(numbers, vec![1, 2, 3, 4, 5, 6]);
}
