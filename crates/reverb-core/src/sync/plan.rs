use super::model::{PlannedEntry, SyncItem, SyncPlan};
use crate::ytdlp::CollectionEntry;
use std::collections::{HashMap, HashSet};

/// Limit refers to remote positions, before unavailable/duplicate entries are filtered.
pub fn plan_sync(
    current: &[SyncItem],
    remote: &[CollectionEntry],
    max_items: Option<u32>,
) -> SyncPlan {
    let old: HashMap<_, _> = current
        .iter()
        .map(|item| (item.source_id.as_str(), item))
        .collect();
    let mut seen = HashSet::new();
    let mut plan = SyncPlan::default();
    for (index, entry) in remote
        .iter()
        .take(max_items.map_or(usize::MAX, |n| n as usize))
        .enumerate()
    {
        if entry
            .title
            .as_deref()
            .is_some_and(|title| matches!(title, "[Private video]" | "[Deleted video]"))
            || entry.id.is_empty()
        {
            plan.unavailable += 1;
            // An unavailable remote entry is still present; retain its local copy.
            if !entry.id.is_empty() {
                seen.insert(entry.id.as_str());
            }
            continue;
        }
        if !seen.insert(entry.id.as_str()) {
            plan.duplicates.push(entry.id.clone());
            continue;
        }
        let item = PlannedEntry {
            entry: entry.clone(),
            position: (index + 1) as u32,
        };
        match old.get(entry.id.as_str()) {
            Some(old) if old.state == "present" => {
                if old.position != item.position {
                    plan.reordered.push(item);
                } else {
                    plan.unchanged.push(item);
                }
            }
            _ => plan.add.push(item),
        }
    }
    plan.removed = current
        .iter()
        .filter(|item| item.state == "present" && !seen.contains(item.source_id.as_str()))
        .map(|item| item.source_id.clone())
        .collect();
    plan
}

#[cfg(test)]
mod tests;
