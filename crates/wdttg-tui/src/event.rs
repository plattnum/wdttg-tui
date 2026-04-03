use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEvent, MouseEvent};
use notify_debouncer_mini::{DebouncedEventKind, new_debouncer};

pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize,
    Tick,
    /// A data file changed externally. Contains the month key (e.g. "2026-04").
    FileChanged(String),
}

pub struct EventHandler {
    rx: mpsc::Receiver<AppEvent>,
    /// Keep the debouncer alive so the watcher thread doesn't stop.
    _debouncer: Option<notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration, data_dir: PathBuf) -> Self {
        let (tx, rx) = mpsc::channel();

        // Crossterm event polling thread
        let crossterm_tx = tx.clone();
        thread::spawn(move || {
            loop {
                if event::poll(tick_rate).unwrap_or(false) {
                    match event::read() {
                        Ok(Event::Key(key)) => {
                            if crossterm_tx.send(AppEvent::Key(key)).is_err() {
                                return;
                            }
                        }
                        Ok(Event::Mouse(mouse)) => {
                            if crossterm_tx.send(AppEvent::Mouse(mouse)).is_err() {
                                return;
                            }
                        }
                        Ok(Event::Resize(_, _)) => {
                            if crossterm_tx.send(AppEvent::Resize).is_err() {
                                return;
                            }
                        }
                        _ => {}
                    }
                } else if crossterm_tx.send(AppEvent::Tick).is_err() {
                    return;
                }
            }
        });

        // File watcher with debouncing
        let debouncer = Self::start_watcher(&data_dir, tx);

        Self {
            rx,
            _debouncer: debouncer,
        }
    }

    fn start_watcher(
        data_dir: &Path,
        tx: mpsc::Sender<AppEvent>,
    ) -> Option<notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>> {
        let debouncer = new_debouncer(
            Duration::from_millis(200),
            move |result: Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
                let events = match result {
                    Ok(events) => events,
                    Err(_) => return,
                };

                for event in events {
                    if event.kind != DebouncedEventKind::Any {
                        continue;
                    }

                    if let Some(month_key) = extract_month_key(&event.path) {
                        let _ = tx.send(AppEvent::FileChanged(month_key));
                    }
                }
            },
        );

        match debouncer {
            Ok(mut debouncer) => {
                // Create data dir if it doesn't exist so we can watch it.
                // FileManager also creates it on first write, but we need it
                // to exist now for the watcher.
                let _ = std::fs::create_dir_all(data_dir);

                if debouncer
                    .watcher()
                    .watch(data_dir, notify::RecursiveMode::NonRecursive)
                    .is_ok()
                {
                    Some(debouncer)
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    pub fn next(&self) -> color_eyre::Result<AppEvent> {
        Ok(self.rx.recv()?)
    }
}

/// Extract month key ("YYYY-MM") from a data file path like ".../2026-04.md".
/// Ignores .tmp, .lock, and other non-markdown files.
fn extract_month_key(path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_str()?;

    // Only care about .md files (not .md.tmp or .md.lock)
    if !file_name.ends_with(".md")
        || file_name.ends_with(".md.tmp")
        || file_name.ends_with(".md.lock")
    {
        return None;
    }

    let stem = file_name.strip_suffix(".md")?;

    // Validate it looks like YYYY-MM
    if stem.len() == 7 && stem.as_bytes()[4] == b'-' {
        let year_part = &stem[..4];
        let month_part = &stem[5..7];
        if year_part.chars().all(|c| c.is_ascii_digit())
            && month_part.chars().all(|c| c.is_ascii_digit())
        {
            return Some(stem.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_month_key_valid() {
        let path = PathBuf::from("/home/user/.wdttg/data/2026-04.md");
        assert_eq!(extract_month_key(&path), Some("2026-04".to_string()));
    }

    #[test]
    fn extract_month_key_ignores_tmp() {
        let path = PathBuf::from("/home/user/.wdttg/data/2026-04.md.tmp");
        assert_eq!(extract_month_key(&path), None);
    }

    #[test]
    fn extract_month_key_ignores_lock() {
        let path = PathBuf::from("/home/user/.wdttg/data/2026-04.md.lock");
        assert_eq!(extract_month_key(&path), None);
    }

    #[test]
    fn extract_month_key_ignores_non_month() {
        let path = PathBuf::from("/home/user/.wdttg/data/notes.md");
        assert_eq!(extract_month_key(&path), None);
    }

    #[test]
    fn extract_month_key_ignores_bad_format() {
        let path = PathBuf::from("/home/user/.wdttg/data/2026-4.md");
        assert_eq!(extract_month_key(&path), None);
    }
}
