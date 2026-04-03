use std::fs::{self, File};
use std::path::{Path, PathBuf};

use fs2::FileExt;

use crate::error::Result;
use crate::model::TimeEntry;
use crate::storage::parser::parse_month_file;
use crate::storage::serializer::serialize_entries;

/// Manages reading and writing monthly markdown files in the data directory.
pub struct FileManager {
    data_dir: PathBuf,
}

impl FileManager {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    /// Path to a monthly file: <data_dir>/YYYY-MM.md
    pub fn month_file_path(&self, month_key: &str) -> PathBuf {
        self.data_dir.join(format!("{month_key}.md"))
    }

    /// Path to the advisory lock file for a month: <data_dir>/YYYY-MM.md.lock
    pub fn lock_file_path(&self, month_key: &str) -> PathBuf {
        self.data_dir.join(format!("{month_key}.md.lock"))
    }

    /// Read entries for a month. Missing file returns empty vec.
    /// Reads are not locked (stale reads handled by cache invalidation).
    pub fn read_month(&self, month_key: &str) -> Result<Vec<TimeEntry>> {
        let path = self.month_file_path(month_key);
        match fs::read_to_string(&path) {
            Ok(content) => parse_month_file(&content),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
            Err(e) => Err(e.into()),
        }
    }

    /// Write entries for a month with atomic write (write .tmp, then rename).
    /// Acquires an exclusive advisory lock before writing, releases after rename.
    /// Creates the data directory if needed.
    pub fn write_month(&self, month_key: &str, entries: &[TimeEntry]) -> Result<()> {
        fs::create_dir_all(&self.data_dir)?;

        let lock_path = self.lock_file_path(month_key);
        let lock_file = File::create(&lock_path)?;
        lock_file.lock_exclusive()?;

        let result = self.write_month_inner(month_key, entries);

        // Always release the lock, even on error
        let _ = lock_file.unlock();
        result
    }

    fn write_month_inner(&self, month_key: &str, entries: &[TimeEntry]) -> Result<()> {
        let path = self.month_file_path(month_key);
        let tmp_path = path.with_extension("md.tmp");
        let content = serialize_entries(month_key, entries);
        fs::write(&tmp_path, &content)?;
        fs::rename(&tmp_path, &path).map_err(|e| {
            let _ = fs::remove_file(&tmp_path);
            e.into()
        })
    }

    /// Check if a monthly file exists on disk.
    pub fn month_exists(&self, month_key: &str) -> bool {
        self.month_file_path(month_key).exists()
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
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

    fn sample_entries() -> Vec<TimeEntry> {
        vec![
            TimeEntry {
                start: dt(2026, 3, 15, 9, 0),
                end: dt(2026, 3, 15, 10, 30),
                description: "Sprint planning".into(),
                client: "acme".into(),
                project: Some("webapp".into()),
                activity: Some("meeting".into()),
                notes: None,
            },
            TimeEntry {
                start: dt(2026, 3, 15, 10, 30),
                end: dt(2026, 3, 15, 12, 0),
                description: "Auth flow".into(),
                client: "acme".into(),
                project: Some("webapp".into()),
                activity: Some("dev".into()),
                notes: Some("See notes.md".into()),
            },
        ]
    }

    #[test]
    fn write_then_read_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let fm = FileManager::new(dir.path().to_path_buf());

        let entries = sample_entries();
        fm.write_month("2026-03", &entries).unwrap();
        let loaded = fm.read_month("2026-03").unwrap();

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].description, "Sprint planning");
        assert_eq!(loaded[1].notes.as_deref(), Some("See notes.md"));
    }

    #[test]
    fn missing_file_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let fm = FileManager::new(dir.path().to_path_buf());
        let entries = fm.read_month("2099-01").unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn month_exists_check() {
        let dir = tempfile::tempdir().unwrap();
        let fm = FileManager::new(dir.path().to_path_buf());

        assert!(!fm.month_exists("2026-03"));
        fm.write_month("2026-03", &sample_entries()).unwrap();
        assert!(fm.month_exists("2026-03"));
    }

    #[test]
    fn no_tmp_left_after_write() {
        let dir = tempfile::tempdir().unwrap();
        let fm = FileManager::new(dir.path().to_path_buf());

        fm.write_month("2026-03", &sample_entries()).unwrap();

        let tmp = dir.path().join("2026-03.md.tmp");
        assert!(!tmp.exists());
    }

    #[test]
    fn creates_data_dir_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("deep").join("data");
        let fm = FileManager::new(nested.clone());

        fm.write_month("2026-03", &sample_entries()).unwrap();
        assert!(nested.join("2026-03.md").exists());
    }

    #[test]
    fn overwrite_existing_month() {
        let dir = tempfile::tempdir().unwrap();
        let fm = FileManager::new(dir.path().to_path_buf());

        fm.write_month("2026-03", &sample_entries()).unwrap();

        let new_entries = vec![TimeEntry {
            start: dt(2026, 3, 20, 14, 0),
            end: dt(2026, 3, 20, 15, 0),
            description: "New entry".into(),
            client: "acme".into(),
            project: None,
            activity: None,
            notes: None,
        }];
        fm.write_month("2026-03", &new_entries).unwrap();

        let loaded = fm.read_month("2026-03").unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].description, "New entry");
    }

    #[test]
    fn lock_file_created_on_write() {
        let dir = tempfile::tempdir().unwrap();
        let fm = FileManager::new(dir.path().to_path_buf());

        fm.write_month("2026-03", &sample_entries()).unwrap();
        assert!(fm.lock_file_path("2026-03").exists());
    }

    #[test]
    fn lock_file_path_format() {
        let dir = tempfile::tempdir().unwrap();
        let fm = FileManager::new(dir.path().to_path_buf());
        let path = fm.lock_file_path("2026-03");
        assert!(path.ends_with("2026-03.md.lock"));
    }

    #[test]
    fn concurrent_writes_dont_corrupt() {
        use std::thread;

        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().to_path_buf();

        // Spawn multiple threads writing to the same month
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let data_dir = data_dir.clone();
                thread::spawn(move || {
                    let fm = FileManager::new(data_dir);
                    let entries = vec![TimeEntry {
                        start: dt(2026, 3, 15, i as u32, 0),
                        end: dt(2026, 3, 15, i as u32 + 1, 0),
                        description: format!("entry {i}"),
                        client: "acme".into(),
                        project: None,
                        activity: None,
                        notes: None,
                    }];
                    fm.write_month("2026-03", &entries).unwrap();
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        // File should be valid (parseable) — some thread won the race
        let fm = FileManager::new(data_dir);
        let entries = fm.read_month("2026-03").unwrap();
        assert_eq!(entries.len(), 1); // last writer wins
        assert!(entries[0].description.starts_with("entry "));
    }
}
