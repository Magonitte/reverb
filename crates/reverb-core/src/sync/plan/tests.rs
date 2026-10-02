use super::*;

fn entry(id: &str) -> CollectionEntry {
    CollectionEntry {
        id: id.into(),
        title: Some(id.into()),
        duration: Some(60.0),
        url: None,
    }
}
fn current(ids: &[&str]) -> Vec<SyncItem> {
    ids.iter()
        .enumerate()
        .map(|(n, id)| SyncItem {
            sync_id: "sync".into(),
            source_id: (*id).into(),
            position: n as u32 + 1,
            title: Some((*id).into()),
            state: "present".into(),
            job_id: None,
            library_id: None,
            job_status: None,
            file_path: None,
            missing: false,
        })
        .collect()
}
#[test]
fn t1_diff_table() {
    // old, remote, limit, added, removed, reordered, unchanged, duplicates
    let cases = [
        (vec![], vec!["a", "b"], None, 2, 0, 0, 0, 0),
        (vec!["a", "b"], vec!["a", "b"], None, 0, 0, 0, 2, 0),
        (vec!["a", "b"], vec!["a", "c", "b"], None, 1, 0, 1, 1, 0),
        (vec!["a", "b"], vec!["a"], None, 0, 1, 0, 1, 0),
        (vec!["a", "b"], vec!["b", "a"], None, 0, 0, 2, 0, 0),
        (vec![], vec!["a", "a", "b"], None, 2, 0, 0, 0, 1),
        (vec![], vec!["a", "b", "c"], Some(2), 2, 0, 0, 0, 0),
        (
            vec!["a", "b", "c"],
            vec!["a", "b", "c"],
            Some(1),
            0,
            2,
            0,
            1,
            0,
        ),
        (vec!["a"], vec![], None, 0, 1, 0, 0, 0),
        (vec![], vec![], None, 0, 0, 0, 0, 0),
        (vec!["a"], vec!["b", "a", "a"], Some(2), 1, 0, 1, 0, 0),
    ];
    for (old, remote, limit, add, removed, reordered, unchanged, duplicates) in cases {
        let plan = plan_sync(
            &current(&old),
            &remote.iter().map(|id| entry(id)).collect::<Vec<_>>(),
            limit,
        );
        assert_eq!(
            (
                plan.add.len(),
                plan.removed.len(),
                plan.reordered.len(),
                plan.unchanged.len(),
                plan.duplicates.len()
            ),
            (add, removed, reordered, unchanged, duplicates),
            "{old:?} -> {remote:?}"
        );
    }
}
#[test]
fn returned_removed_and_failed_items_are_distinct() {
    let mut old = current(&["a", "b"]);
    old[0].state = "removed".into();
    old[1].job_status = Some("failed".into());
    let plan = plan_sync(&old, &[entry("a"), entry("b")], None);
    assert_eq!(plan.add[0].entry.id, "a");
    assert_eq!(plan.unchanged[0].entry.id, "b");
    assert!(plan.removed.is_empty());
}
#[test]
fn unavailable_positions_count_toward_limit_and_duplicates_keep_first() {
    let mut private = entry("private");
    private.title = Some("[Private video]".into());
    let plan = plan_sync(&[], &[private, entry("a"), entry("a"), entry("b")], Some(3));
    assert_eq!(plan.unavailable, 1);
    assert_eq!(plan.add.len(), 1);
    assert_eq!(plan.add[0].position, 2);
    assert_eq!(plan.duplicates, vec!["a"]);
}

#[test]
fn private_remote_entry_preserves_existing_local_track() {
    let mut private = entry("a");
    private.title = Some("[Private video]".into());
    let plan = plan_sync(&current(&["a"]), &[private], None);
    assert_eq!(plan.unavailable, 1);
    assert!(plan.removed.is_empty());
    assert!(plan.add.is_empty());
}
