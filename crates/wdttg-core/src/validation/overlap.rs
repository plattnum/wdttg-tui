use chrono::NaiveDateTime;

use crate::model::TimeEntry;

#[derive(Debug, Clone, PartialEq)]
pub enum OverlapType {
    /// new.start falls inside an existing entry
    StartOverlap,
    /// new.end falls inside an existing entry
    EndOverlap,
    /// new fully contains an existing entry
    Encompassed,
}

#[derive(Debug, Clone)]
pub struct OverlapInfo {
    pub overlap_type: OverlapType,
    pub conflicting_entry: TimeEntry,
}

#[derive(Debug)]
pub struct OverlapResult {
    pub has_overlaps: bool,
    pub overlaps: Vec<OverlapInfo>,
}

/// Detect overlaps between a proposed (start, end) and existing entries.
/// If `exclude` is provided, that entry is skipped (for edit scenarios).
pub fn find_overlaps(
    start: NaiveDateTime,
    end: NaiveDateTime,
    existing: &[TimeEntry],
    exclude: Option<&TimeEntry>,
) -> OverlapResult {
    let mut overlaps = Vec::new();

    for entry in existing {
        // Skip the entry being edited
        if let Some(exc) = exclude
            && entry.start == exc.start
            && entry.end == exc.end
        {
            continue;
        }

        // Overlap condition: new.start < existing.end AND new.end > existing.start
        if start < entry.end && end > entry.start {
            let start_inside = start >= entry.start && start < entry.end;
            let end_inside = end > entry.start && end <= entry.end;

            let overlap_type = if start_inside {
                OverlapType::StartOverlap
            } else if end_inside {
                OverlapType::EndOverlap
            } else {
                OverlapType::Encompassed
            };

            overlaps.push(OverlapInfo {
                overlap_type,
                conflicting_entry: entry.clone(),
            });
        }
    }

    OverlapResult {
        has_overlaps: !overlaps.is_empty(),
        overlaps,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn dt(y: i32, m: u32, d: u32, h: u32, min: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
    }

    fn entry(sh: u32, sm: u32, eh: u32, em: u32) -> TimeEntry {
        TimeEntry {
            start: dt(2026, 3, 15, sh, sm),
            end: dt(2026, 3, 15, eh, em),
            description: "existing".into(),
            client: "acme".into(),
            project: None,
            activity: None,
            notes: None,
        }
    }

    #[test]
    fn adjacent_no_overlap() {
        // 10:00-11:00 and 11:00-12:00 should NOT overlap
        let existing = vec![entry(10, 0, 11, 0)];
        let result = find_overlaps(
            dt(2026, 3, 15, 11, 0),
            dt(2026, 3, 15, 12, 0),
            &existing,
            None,
        );
        assert!(!result.has_overlaps);
    }

    #[test]
    fn adjacent_before_no_overlap() {
        let existing = vec![entry(11, 0, 12, 0)];
        let result = find_overlaps(
            dt(2026, 3, 15, 10, 0),
            dt(2026, 3, 15, 11, 0),
            &existing,
            None,
        );
        assert!(!result.has_overlaps);
    }

    #[test]
    fn start_overlap() {
        // new 09:30-10:30 vs existing 09:00-10:00
        let existing = vec![entry(9, 0, 10, 0)];
        let result = find_overlaps(
            dt(2026, 3, 15, 9, 30),
            dt(2026, 3, 15, 10, 30),
            &existing,
            None,
        );
        assert!(result.has_overlaps);
        assert_eq!(result.overlaps[0].overlap_type, OverlapType::StartOverlap);
    }

    #[test]
    fn end_overlap() {
        // new 09:30-10:30 vs existing 10:00-11:00
        let existing = vec![entry(10, 0, 11, 0)];
        let result = find_overlaps(
            dt(2026, 3, 15, 9, 30),
            dt(2026, 3, 15, 10, 30),
            &existing,
            None,
        );
        assert!(result.has_overlaps);
        assert_eq!(result.overlaps[0].overlap_type, OverlapType::EndOverlap);
    }

    #[test]
    fn encompassed() {
        // new 08:00-12:00 vs existing 09:00-10:00
        let existing = vec![entry(9, 0, 10, 0)];
        let result = find_overlaps(
            dt(2026, 3, 15, 8, 0),
            dt(2026, 3, 15, 12, 0),
            &existing,
            None,
        );
        assert!(result.has_overlaps);
        assert_eq!(result.overlaps[0].overlap_type, OverlapType::Encompassed);
    }

    #[test]
    fn exact_same_time() {
        let existing = vec![entry(9, 0, 10, 0)];
        let result = find_overlaps(
            dt(2026, 3, 15, 9, 0),
            dt(2026, 3, 15, 10, 0),
            &existing,
            None,
        );
        assert!(result.has_overlaps);
        assert_eq!(result.overlaps[0].overlap_type, OverlapType::StartOverlap);
    }

    #[test]
    fn exclude_self_when_editing() {
        let existing_entry = entry(9, 0, 10, 0);
        let existing = vec![existing_entry.clone()];
        let result = find_overlaps(
            dt(2026, 3, 15, 9, 0),
            dt(2026, 3, 15, 10, 30),
            &existing,
            Some(&existing_entry),
        );
        assert!(!result.has_overlaps);
    }

    #[test]
    fn multiple_overlaps() {
        let existing = vec![entry(9, 0, 10, 0), entry(11, 0, 12, 0)];
        // new 08:00-13:00 encompasses both
        let result = find_overlaps(
            dt(2026, 3, 15, 8, 0),
            dt(2026, 3, 15, 13, 0),
            &existing,
            None,
        );
        assert!(result.has_overlaps);
        assert_eq!(result.overlaps.len(), 2);
    }

    #[test]
    fn midnight_spanning_overlap() {
        // Existing: 23:00-02:00 next day
        let existing = vec![TimeEntry {
            start: dt(2026, 3, 15, 23, 0),
            end: dt(2026, 3, 16, 2, 0),
            description: "night".into(),
            client: "acme".into(),
            project: None,
            activity: None,
            notes: None,
        }];
        // New: 01:00-03:00 on the 16th overlaps
        let result = find_overlaps(
            dt(2026, 3, 16, 1, 0),
            dt(2026, 3, 16, 3, 0),
            &existing,
            None,
        );
        assert!(result.has_overlaps);
    }
}
