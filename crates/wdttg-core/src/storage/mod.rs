pub mod cache;
pub mod file_manager;
pub mod parser;
pub mod serializer;

use chrono::NaiveDateTime;

use crate::config::AppConfig;
use crate::error::{Error, Result};
use crate::model::{DateRange, EntryFilter, NewEntry, TimeEntry};
use crate::validation::{find_overlaps, validate_new_entry};

use self::cache::MonthCache;
use self::file_manager::FileManager;

/// Load a month's entries, using cache if available.
pub fn load_month(
    month_key: &str,
    fm: &FileManager,
    cache: &mut MonthCache,
) -> Result<Vec<TimeEntry>> {
    if let Some(entries) = cache.get(month_key) {
        return Ok(entries.clone());
    }
    let entries = fm.read_month(month_key)?;
    cache.insert(month_key, entries.clone());
    Ok(entries)
}

/// Load all entries within a date range, spanning multiple months if needed.
pub fn load_date_range(
    range: &DateRange,
    fm: &FileManager,
    cache: &mut MonthCache,
) -> Result<Vec<TimeEntry>> {
    let months = range.months_spanned();
    let mut all_entries = Vec::new();

    for month_key in &months {
        let entries = load_month(month_key, fm, cache)?;
        for entry in entries {
            // Include if entry's start date falls within range
            let entry_date = entry.start.date();
            if entry_date >= range.start && entry_date <= range.end {
                all_entries.push(entry);
            }
        }
    }

    all_entries.sort_by_key(|e| e.start);
    Ok(all_entries)
}

/// Load entries within a date range, applying a filter.
pub fn load_filtered(
    range: &DateRange,
    filter: &EntryFilter,
    fm: &FileManager,
    cache: &mut MonthCache,
) -> Result<Vec<TimeEntry>> {
    let entries = load_date_range(range, fm, cache)?;
    Ok(entries.into_iter().filter(|e| filter.matches(e)).collect())
}

/// Find an entry by its computed entry_id within a date range.
pub fn find_entry_by_id(
    entry_id: &str,
    range: &DateRange,
    fm: &FileManager,
    cache: &mut MonthCache,
) -> Result<TimeEntry> {
    let entries = load_date_range(range, fm, cache)?;
    entries
        .into_iter()
        .find(|e| e.entry_id() == entry_id)
        .ok_or(Error::NotFound)
}

/// Create a new time entry. Validates, checks overlaps, writes to disk.
pub fn create_entry(
    new: NewEntry,
    config: &AppConfig,
    fm: &FileManager,
    cache: &mut MonthCache,
) -> Result<TimeEntry> {
    validate_new_entry(&new, config)?;

    let month_key = new.start.format("%Y-%m").to_string();
    let mut entries = load_month(&month_key, fm, cache)?;

    let overlap_result = find_overlaps(new.start, new.end, &entries, None);
    if overlap_result.has_overlaps {
        let conflict = &overlap_result.overlaps[0].conflicting_entry;
        return Err(Error::Overlap(format!(
            "conflicts with {} - {} ({})",
            conflict.start.format("%H:%M"),
            conflict.end.format("%H:%M"),
            conflict.description
        )));
    }

    let entry = TimeEntry::from(new);
    entries.push(entry.clone());
    entries.sort_by_key(|e| e.start);
    fm.write_month(&month_key, &entries)?;
    cache.invalidate(&month_key);

    Ok(entry)
}

/// Update an existing entry identified by (original_start, original_end).
/// Handles month boundary changes if the start month changes.
pub fn update_entry(
    original_start: NaiveDateTime,
    original_end: NaiveDateTime,
    updated: NewEntry,
    config: &AppConfig,
    fm: &FileManager,
    cache: &mut MonthCache,
) -> Result<TimeEntry> {
    validate_new_entry(&updated, config)?;

    let old_month = original_start.format("%Y-%m").to_string();
    let new_month = updated.start.format("%Y-%m").to_string();

    // Load old month and find the entry
    let mut old_entries = load_month(&old_month, fm, cache)?;
    let old_idx =
        find_entry_index(&old_entries, original_start, original_end).ok_or(Error::NotFound)?;
    let old_entry = old_entries[old_idx].clone();

    if old_month == new_month {
        // Same month: check overlaps excluding self, replace in place
        let overlap_result =
            find_overlaps(updated.start, updated.end, &old_entries, Some(&old_entry));
        if overlap_result.has_overlaps {
            let conflict = &overlap_result.overlaps[0].conflicting_entry;
            return Err(Error::Overlap(format!(
                "conflicts with {} - {} ({})",
                conflict.start.format("%H:%M"),
                conflict.end.format("%H:%M"),
                conflict.description
            )));
        }

        let new_entry = TimeEntry::from(updated);
        old_entries[old_idx] = new_entry.clone();
        old_entries.sort_by_key(|e| e.start);
        fm.write_month(&old_month, &old_entries)?;
        cache.invalidate(&old_month);
        Ok(new_entry)
    } else {
        // Month changed: remove from old, add to new
        old_entries.remove(old_idx);
        fm.write_month(&old_month, &old_entries)?;
        cache.invalidate(&old_month);

        let mut new_entries = load_month(&new_month, fm, cache)?;
        let overlap_result = find_overlaps(updated.start, updated.end, &new_entries, None);
        if overlap_result.has_overlaps {
            // Rollback: put old entry back
            old_entries.push(old_entry);
            old_entries.sort_by_key(|e| e.start);
            fm.write_month(&old_month, &old_entries)?;
            cache.invalidate(&old_month);

            let conflict = &overlap_result.overlaps[0].conflicting_entry;
            return Err(Error::Overlap(format!(
                "conflicts with {} - {} ({})",
                conflict.start.format("%H:%M"),
                conflict.end.format("%H:%M"),
                conflict.description
            )));
        }

        let new_entry = TimeEntry::from(updated);
        new_entries.push(new_entry.clone());
        new_entries.sort_by_key(|e| e.start);
        fm.write_month(&new_month, &new_entries)?;
        cache.invalidate(&new_month);
        Ok(new_entry)
    }
}

/// Delete an entry identified by (start, end).
pub fn delete_entry(
    start: NaiveDateTime,
    end: NaiveDateTime,
    fm: &FileManager,
    cache: &mut MonthCache,
) -> Result<()> {
    let month_key = start.format("%Y-%m").to_string();
    let mut entries = load_month(&month_key, fm, cache)?;
    let idx = find_entry_index(&entries, start, end).ok_or(Error::NotFound)?;
    entries.remove(idx);
    fm.write_month(&month_key, &entries)?;
    cache.invalidate(&month_key);
    Ok(())
}

fn find_entry_index(
    entries: &[TimeEntry],
    start: NaiveDateTime,
    end: NaiveDateTime,
) -> Option<usize> {
    entries
        .iter()
        .position(|e| e.start == start && e.end == end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn dt(y: i32, m: u32, d: u32, h: u32, min: u32) -> chrono::NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
    }

    fn setup() -> (tempfile::TempDir, FileManager, MonthCache) {
        let dir = tempfile::tempdir().unwrap();
        let fm = FileManager::new(dir.path().to_path_buf());
        let cache = MonthCache::new(24);
        (dir, fm, cache)
    }

    #[test]
    fn load_month_caches_result() {
        let (_dir, fm, mut cache) = setup();

        let entries = vec![TimeEntry {
            start: dt(2026, 3, 15, 9, 0),
            end: dt(2026, 3, 15, 10, 0),
            description: "Work".into(),
            client: "acme".into(),
            project: None,
            activity: None,
            notes: None,
        }];
        fm.write_month("2026-03", &entries).unwrap();

        // First load: reads from disk
        let result = load_month("2026-03", &fm, &mut cache).unwrap();
        assert_eq!(result.len(), 1);

        // Second load: comes from cache (even if we deleted the file)
        std::fs::remove_file(fm.month_file_path("2026-03")).unwrap();
        let cached = load_month("2026-03", &fm, &mut cache).unwrap();
        assert_eq!(cached.len(), 1);
    }

    #[test]
    fn load_date_range_single_month() {
        let (_dir, fm, mut cache) = setup();

        let entries = vec![
            TimeEntry {
                start: dt(2026, 3, 10, 9, 0),
                end: dt(2026, 3, 10, 10, 0),
                description: "Early".into(),
                client: "acme".into(),
                project: None,
                activity: None,
                notes: None,
            },
            TimeEntry {
                start: dt(2026, 3, 20, 9, 0),
                end: dt(2026, 3, 20, 10, 0),
                description: "Late".into(),
                client: "acme".into(),
                project: None,
                activity: None,
                notes: None,
            },
        ];
        fm.write_month("2026-03", &entries).unwrap();

        // Range only covers 15-25, should exclude the 10th entry
        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 25).unwrap(),
        );
        let result = load_date_range(&range, &fm, &mut cache).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Late");
    }

    #[test]
    fn load_date_range_multi_month() {
        let (_dir, fm, mut cache) = setup();

        let march = vec![TimeEntry {
            start: dt(2026, 3, 28, 9, 0),
            end: dt(2026, 3, 28, 10, 0),
            description: "March".into(),
            client: "acme".into(),
            project: None,
            activity: None,
            notes: None,
        }];
        let april = vec![TimeEntry {
            start: dt(2026, 4, 2, 9, 0),
            end: dt(2026, 4, 2, 10, 0),
            description: "April".into(),
            client: "acme".into(),
            project: None,
            activity: None,
            notes: None,
        }];
        fm.write_month("2026-03", &march).unwrap();
        fm.write_month("2026-04", &april).unwrap();

        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2026, 3, 25).unwrap(),
            NaiveDate::from_ymd_opt(2026, 4, 5).unwrap(),
        );
        let result = load_date_range(&range, &fm, &mut cache).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].description, "March");
        assert_eq!(result[1].description, "April");
    }

    #[test]
    fn load_date_range_empty_months() {
        let (_dir, fm, mut cache) = setup();

        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2099, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2099, 1, 31).unwrap(),
        );
        let result = load_date_range(&range, &fm, &mut cache).unwrap();
        assert!(result.is_empty());
    }

    fn test_config() -> AppConfig {
        use crate::config::Preferences;
        use crate::model::{Activity, Client, Project};

        AppConfig {
            preferences: Preferences::default(),
            bill_from: Default::default(),
            clients: vec![Client {
                id: "acme".into(),
                name: "Acme".into(),
                color: "#FF0000".into(),
                rate: 100.0,
                currency: "USD".into(),
                archived: false,
                address: None,
                email: None,
                tax_id: None,
                payment_terms: None,
                notes: None,
                projects: vec![Project {
                    id: "webapp".into(),
                    name: "Web App".into(),
                    color: "#00FF00".into(),
                    rate_override: None,
                    archived: false,
                }],
                activities: vec![Activity {
                    id: "dev".into(),
                    name: "Dev".into(),
                    color: "#0000FF".into(),
                    archived: false,
                }],
            }],
        }
    }

    fn new_entry(sh: u32, sm: u32, eh: u32, em: u32, desc: &str) -> NewEntry {
        NewEntry {
            start: dt(2026, 3, 15, sh, sm),
            end: dt(2026, 3, 15, eh, em),
            description: desc.into(),
            client: "acme".into(),
            project: None,
            activity: None,
            notes: None,
        }
    }

    #[test]
    fn create_entry_persists() {
        let (_dir, fm, mut cache) = setup();
        let config = test_config();

        let entry =
            create_entry(new_entry(9, 0, 10, 0, "Morning"), &config, &fm, &mut cache).unwrap();
        assert_eq!(entry.description, "Morning");

        let loaded = fm.read_month("2026-03").unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].description, "Morning");
    }

    #[test]
    fn create_entry_rejects_overlap() {
        let (_dir, fm, mut cache) = setup();
        let config = test_config();

        create_entry(new_entry(9, 0, 10, 0, "First"), &config, &fm, &mut cache).unwrap();
        let result = create_entry(
            new_entry(9, 30, 10, 30, "Overlap"),
            &config,
            &fm,
            &mut cache,
        );
        assert!(matches!(result, Err(Error::Overlap(_))));
    }

    #[test]
    fn create_entry_rejects_invalid_client() {
        let (_dir, fm, mut cache) = setup();
        let config = test_config();

        let mut entry = new_entry(9, 0, 10, 0, "Work");
        entry.client = "nonexistent".into();
        let result = create_entry(entry, &config, &fm, &mut cache);
        assert!(matches!(result, Err(Error::Validation(_))));
    }

    #[test]
    fn update_entry_same_month() {
        let (_dir, fm, mut cache) = setup();
        let config = test_config();

        let original =
            create_entry(new_entry(9, 0, 10, 0, "Original"), &config, &fm, &mut cache).unwrap();
        let updated = update_entry(
            original.start,
            original.end,
            new_entry(9, 0, 10, 30, "Updated"),
            &config,
            &fm,
            &mut cache,
        )
        .unwrap();
        assert_eq!(updated.description, "Updated");
        assert_eq!(updated.duration_minutes(), 90);

        let loaded = fm.read_month("2026-03").unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].description, "Updated");
    }

    #[test]
    fn update_entry_rejects_overlap_with_other() {
        let (_dir, fm, mut cache) = setup();
        let config = test_config();

        let first =
            create_entry(new_entry(9, 0, 10, 0, "First"), &config, &fm, &mut cache).unwrap();
        create_entry(new_entry(11, 0, 12, 0, "Second"), &config, &fm, &mut cache).unwrap();

        // Try to move first to overlap with second
        let result = update_entry(
            first.start,
            first.end,
            new_entry(11, 0, 12, 30, "Moved"),
            &config,
            &fm,
            &mut cache,
        );
        assert!(matches!(result, Err(Error::Overlap(_))));
    }

    #[test]
    fn update_entry_month_boundary() {
        let (_dir, fm, mut cache) = setup();
        let config = test_config();

        let original =
            create_entry(new_entry(9, 0, 10, 0, "March"), &config, &fm, &mut cache).unwrap();

        // Move to April
        let april_entry = NewEntry {
            start: dt(2026, 4, 1, 9, 0),
            end: dt(2026, 4, 1, 10, 0),
            description: "April".into(),
            client: "acme".into(),
            project: None,
            activity: None,
            notes: None,
        };
        update_entry(
            original.start,
            original.end,
            april_entry,
            &config,
            &fm,
            &mut cache,
        )
        .unwrap();

        let march = fm.read_month("2026-03").unwrap();
        let april = fm.read_month("2026-04").unwrap();
        assert!(march.is_empty());
        assert_eq!(april.len(), 1);
        assert_eq!(april[0].description, "April");
    }

    #[test]
    fn delete_entry_removes() {
        let (_dir, fm, mut cache) = setup();
        let config = test_config();

        let entry =
            create_entry(new_entry(9, 0, 10, 0, "Doomed"), &config, &fm, &mut cache).unwrap();
        delete_entry(entry.start, entry.end, &fm, &mut cache).unwrap();

        let loaded = fm.read_month("2026-03").unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn delete_nonexistent_returns_not_found() {
        let (_dir, fm, mut cache) = setup();

        let result = delete_entry(
            dt(2026, 3, 15, 9, 0),
            dt(2026, 3, 15, 10, 0),
            &fm,
            &mut cache,
        );
        assert!(matches!(result, Err(Error::NotFound)));
    }

    #[test]
    fn full_crud_cycle() {
        let (_dir, fm, mut cache) = setup();
        let config = test_config();

        // Create
        let entry =
            create_entry(new_entry(9, 0, 10, 0, "Created"), &config, &fm, &mut cache).unwrap();
        assert_eq!(load_month("2026-03", &fm, &mut cache).unwrap().len(), 1);

        // Update
        let updated = update_entry(
            entry.start,
            entry.end,
            new_entry(9, 0, 11, 0, "Updated"),
            &config,
            &fm,
            &mut cache,
        )
        .unwrap();
        let loaded = fm.read_month("2026-03").unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].description, "Updated");

        // Delete
        delete_entry(updated.start, updated.end, &fm, &mut cache).unwrap();
        assert!(fm.read_month("2026-03").unwrap().is_empty());
    }

    // --- EntryFilter / load_filtered tests (WDTTG-017) ---

    fn make_entries() -> Vec<TimeEntry> {
        vec![
            TimeEntry {
                start: dt(2026, 3, 15, 9, 0),
                end: dt(2026, 3, 15, 10, 0),
                description: "Sprint planning".into(),
                client: "acme".into(),
                project: Some("webapp".into()),
                activity: Some("meeting".into()),
                notes: None,
            },
            TimeEntry {
                start: dt(2026, 3, 15, 10, 0),
                end: dt(2026, 3, 15, 12, 0),
                description: "Auth flow implementation".into(),
                client: "acme".into(),
                project: Some("webapp".into()),
                activity: Some("dev".into()),
                notes: None,
            },
            TimeEntry {
                start: dt(2026, 3, 15, 13, 0),
                end: dt(2026, 3, 15, 14, 0),
                description: "Logo design review".into(),
                client: "globex".into(),
                project: Some("branding".into()),
                activity: Some("meeting".into()),
                notes: None,
            },
        ]
    }

    #[test]
    fn load_filtered_empty_filter_returns_all() {
        let (_dir, fm, mut cache) = setup();
        fm.write_month("2026-03", &make_entries()).unwrap();

        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
        );
        let filter = EntryFilter::default();
        let result = load_filtered(&range, &filter, &fm, &mut cache).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn load_filtered_by_client() {
        let (_dir, fm, mut cache) = setup();
        fm.write_month("2026-03", &make_entries()).unwrap();

        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
        );
        let filter = EntryFilter {
            client: Some("acme".into()),
            ..Default::default()
        };
        let result = load_filtered(&range, &filter, &fm, &mut cache).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|e| e.client == "acme"));
    }

    #[test]
    fn load_filtered_by_activity() {
        let (_dir, fm, mut cache) = setup();
        fm.write_month("2026-03", &make_entries()).unwrap();

        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
        );
        let filter = EntryFilter {
            activity: Some("meeting".into()),
            ..Default::default()
        };
        let result = load_filtered(&range, &filter, &fm, &mut cache).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn load_filtered_by_description_case_insensitive() {
        let (_dir, fm, mut cache) = setup();
        fm.write_month("2026-03", &make_entries()).unwrap();

        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
        );
        let filter = EntryFilter {
            description_contains: Some("AUTH".into()),
            ..Default::default()
        };
        let result = load_filtered(&range, &filter, &fm, &mut cache).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Auth flow implementation");
    }

    #[test]
    fn load_filtered_multiple_fields_and() {
        let (_dir, fm, mut cache) = setup();
        fm.write_month("2026-03", &make_entries()).unwrap();

        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
        );
        let filter = EntryFilter {
            client: Some("acme".into()),
            activity: Some("meeting".into()),
            ..Default::default()
        };
        let result = load_filtered(&range, &filter, &fm, &mut cache).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Sprint planning");
    }

    // --- find_entry_by_id tests (WDTTG-018) ---

    #[test]
    fn find_entry_by_id_found() {
        let (_dir, fm, mut cache) = setup();
        let entries = make_entries();
        let expected_id = entries[0].entry_id();
        fm.write_month("2026-03", &entries).unwrap();

        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
        );
        let found = find_entry_by_id(&expected_id, &range, &fm, &mut cache).unwrap();
        assert_eq!(found.description, "Sprint planning");
    }

    #[test]
    fn find_entry_by_id_not_found() {
        let (_dir, fm, mut cache) = setup();
        fm.write_month("2026-03", &make_entries()).unwrap();

        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
        );
        let result = find_entry_by_id("e_00000000", &range, &fm, &mut cache);
        assert!(matches!(result, Err(Error::NotFound)));
    }

    #[test]
    fn find_entry_by_id_roundtrip() {
        let (_dir, fm, mut cache) = setup();
        let config = test_config();

        let entry =
            create_entry(new_entry(9, 0, 10, 0, "Findable"), &config, &fm, &mut cache).unwrap();
        let id = entry.entry_id();

        cache.invalidate_all();
        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
        );
        let found = find_entry_by_id(&id, &range, &fm, &mut cache).unwrap();
        assert_eq!(found.description, "Findable");
        assert_eq!(found.start, entry.start);
        assert_eq!(found.end, entry.end);
    }
}
