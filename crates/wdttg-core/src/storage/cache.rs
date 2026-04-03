use lru::LruCache;
use std::num::NonZeroUsize;

use crate::model::TimeEntry;

const DEFAULT_MAX_MONTHS: usize = 24;

/// LRU cache for loaded month data, capped at max_months.
pub struct MonthCache {
    cache: LruCache<String, Vec<TimeEntry>>,
}

impl MonthCache {
    pub fn new(max_months: usize) -> Self {
        Self {
            cache: LruCache::new(
                NonZeroUsize::new(max_months)
                    .unwrap_or(NonZeroUsize::new(DEFAULT_MAX_MONTHS).unwrap()),
            ),
        }
    }

    /// Get entries for a month, updating LRU access order.
    pub fn get(&mut self, month_key: &str) -> Option<&Vec<TimeEntry>> {
        self.cache.get(month_key)
    }

    /// Insert entries for a month. Evicts LRU entry if at capacity.
    pub fn insert(&mut self, month_key: &str, entries: Vec<TimeEntry>) {
        self.cache.put(month_key.to_string(), entries);
    }

    /// Remove a specific month from cache.
    pub fn invalidate(&mut self, month_key: &str) {
        self.cache.pop(month_key);
    }

    /// Clear the entire cache.
    pub fn invalidate_all(&mut self) {
        self.cache.clear();
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

impl Default for MonthCache {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_MONTHS)
    }
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

    fn dummy_entries(n: usize) -> Vec<TimeEntry> {
        (0..n)
            .map(|i| TimeEntry {
                start: dt(2026, 1, 1, i as u32, 0),
                end: dt(2026, 1, 1, i as u32 + 1, 0),
                description: format!("entry {i}"),
                client: "acme".into(),
                project: None,
                activity: None,
                notes: None,
            })
            .collect()
    }

    #[test]
    fn cache_hit_and_miss() {
        let mut cache = MonthCache::new(5);
        assert!(cache.get("2026-03").is_none());

        cache.insert("2026-03", dummy_entries(1));
        assert!(cache.get("2026-03").is_some());
        assert_eq!(cache.get("2026-03").unwrap().len(), 1);
    }

    #[test]
    fn lru_eviction_at_capacity() {
        let mut cache = MonthCache::new(3);
        cache.insert("2026-01", dummy_entries(1));
        cache.insert("2026-02", dummy_entries(1));
        cache.insert("2026-03", dummy_entries(1));

        // Cache is full (3). Insert a 4th, oldest (2026-01) should be evicted.
        cache.insert("2026-04", dummy_entries(1));
        assert!(cache.get("2026-01").is_none());
        assert!(cache.get("2026-02").is_some());
        assert!(cache.get("2026-04").is_some());
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn access_updates_lru_order() {
        let mut cache = MonthCache::new(3);
        cache.insert("2026-01", dummy_entries(1));
        cache.insert("2026-02", dummy_entries(1));
        cache.insert("2026-03", dummy_entries(1));

        // Access 2026-01, making 2026-02 the LRU
        let _ = cache.get("2026-01");

        // Insert 4th, 2026-02 (now LRU) should be evicted
        cache.insert("2026-04", dummy_entries(1));
        assert!(cache.get("2026-01").is_some());
        assert!(cache.get("2026-02").is_none());
    }

    #[test]
    fn invalidate_single() {
        let mut cache = MonthCache::new(5);
        cache.insert("2026-03", dummy_entries(1));
        cache.invalidate("2026-03");
        assert!(cache.get("2026-03").is_none());
        assert!(cache.is_empty());
    }

    #[test]
    fn invalidate_all() {
        let mut cache = MonthCache::new(5);
        cache.insert("2026-01", dummy_entries(1));
        cache.insert("2026-02", dummy_entries(1));
        cache.invalidate_all();
        assert!(cache.is_empty());
    }

    #[test]
    fn default_capacity_is_24() {
        let cache = MonthCache::default();
        assert_eq!(cache.cache.cap().get(), 24);
    }
}
