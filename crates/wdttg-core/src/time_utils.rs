use chrono::{NaiveDateTime, NaiveTime, Timelike};

use crate::model::TimeEntry;

/// Snap a datetime to the nearest N-minute grid boundary.
/// snap_minutes of 0 returns the datetime unchanged.
pub fn snap_to_grid(dt: NaiveDateTime, snap_minutes: u32) -> NaiveDateTime {
    if snap_minutes == 0 {
        return dt;
    }
    let minute = dt.minute();
    let remainder = minute % snap_minutes;
    let half = snap_minutes / 2;
    let snapped_minute = if remainder < half + (snap_minutes % 2) {
        minute - remainder
    } else {
        minute + (snap_minutes - remainder)
    };
    if snapped_minute >= 60 {
        // Rolled over to next hour
        dt.with_minute(0).unwrap().with_second(0).unwrap() + chrono::Duration::hours(1)
    } else {
        dt.with_minute(snapped_minute)
            .unwrap()
            .with_second(0)
            .unwrap()
    }
}

/// Format a duration in minutes as a human-readable string.
/// "Xh Ym" if >= 60, "Ym" if < 60, "0m" for zero.
pub fn format_duration(minutes: i64) -> String {
    if minutes == 0 {
        return "0m".into();
    }
    let negative = minutes < 0;
    let abs = minutes.unsigned_abs();
    let h = abs / 60;
    let m = abs % 60;
    let sign = if negative { "-" } else { "" };
    if h > 0 && m > 0 {
        format!("{sign}{h}h {m}m")
    } else if h > 0 {
        format!("{sign}{h}h")
    } else {
        format!("{sign}{m}m")
    }
}

/// Parse a duration string into minutes. Supports:
/// "1h 30m", "1h30m", "90m", "1.5h", "1,5h", "1:30", "2h"
pub fn parse_duration(input: &str) -> Option<i64> {
    let s = input.trim().to_lowercase();
    if s.is_empty() {
        return None;
    }

    // "H:MM" format
    if let Some((h, m)) = s.split_once(':') {
        let hours: i64 = h.trim().parse().ok()?;
        let mins: i64 = m.trim().parse().ok()?;
        return Some(hours * 60 + mins);
    }

    // "Xh Ym" or "Xh" or "Ym"
    let s = s.replace(',', ".");
    if s.contains('h') || s.contains('m') {
        let mut total: f64 = 0.0;
        let normalized = s.replace(' ', "");

        if normalized.contains('h') {
            let parts: Vec<&str> = normalized.split('h').collect();
            let h: f64 = parts[0].parse().ok()?;
            total += h * 60.0;
            if parts.len() > 1 {
                let m_str = parts[1].trim_end_matches('m');
                if !m_str.is_empty() {
                    let m: f64 = m_str.parse().ok()?;
                    total += m;
                }
            }
        } else {
            // Minutes only: "90m"
            let m_str = normalized.trim_end_matches('m');
            let m: f64 = m_str.parse().ok()?;
            total = m;
        }

        return Some(total.round() as i64);
    }

    // Plain number = minutes
    s.parse::<i64>().ok()
}

/// Format a time for display, respecting 12h/24h preference.
pub fn format_time(time: NaiveTime, format_24h: bool) -> String {
    if format_24h {
        time.format("%H:%M").to_string()
    } else {
        let hour = time.hour();
        let minute = time.minute();
        let (h12, period) = if hour == 0 {
            (12, "AM")
        } else if hour < 12 {
            (hour, "AM")
        } else if hour == 12 {
            (12, "PM")
        } else {
            (hour - 12, "PM")
        };
        format!("{h12}:{minute:02} {period}")
    }
}

/// An available (unoccupied) time slot within a queried range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableSlot {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub duration_minutes: i64,
}

/// Compute available time slots within a range by finding gaps between entries.
///
/// `entries` should be sorted by start time. Entries partially overlapping the
/// range boundaries are clipped. If `min_duration_minutes` is provided, slots
/// shorter than that threshold are excluded.
pub fn compute_available_slots(
    range_start: NaiveDateTime,
    range_end: NaiveDateTime,
    entries: &[TimeEntry],
    min_duration_minutes: Option<i64>,
) -> Vec<AvailableSlot> {
    let min_dur = min_duration_minutes.unwrap_or(0);
    let mut slots = Vec::new();
    let mut cursor = range_start;

    for entry in entries {
        // Skip entries entirely outside the range
        if entry.end <= range_start || entry.start >= range_end {
            continue;
        }

        // Clip entry to range boundaries
        let occupied_start = entry.start.max(range_start);

        // Gap before this entry
        if occupied_start > cursor {
            let duration = (occupied_start - cursor).num_minutes();
            if duration >= min_dur {
                slots.push(AvailableSlot {
                    start: cursor,
                    end: occupied_start,
                    duration_minutes: duration,
                });
            }
        }

        // Advance cursor past this entry (clipped to range)
        let occupied_end = entry.end.min(range_end);
        if occupied_end > cursor {
            cursor = occupied_end;
        }
    }

    // Gap after the last entry
    if cursor < range_end {
        let duration = (range_end - cursor).num_minutes();
        if duration >= min_dur {
            slots.push(AvailableSlot {
                start: cursor,
                end: range_end,
                duration_minutes: duration,
            });
        }
    }

    slots
}

/// Adjacent entries around a target time window.
pub struct AdjacentEntries {
    pub previous: Option<TimeEntry>,
    pub next: Option<TimeEntry>,
}

/// Find entries adjacent to a target time window.
/// Previous: entry whose end is closest to (but <=) target_start.
/// Next: entry whose start is closest to (but >=) target_end.
pub fn find_adjacent(
    target_start: NaiveDateTime,
    target_end: NaiveDateTime,
    entries: &[TimeEntry],
    exclude: Option<&TimeEntry>,
) -> AdjacentEntries {
    let mut previous: Option<&TimeEntry> = None;
    let mut next: Option<&TimeEntry> = None;

    for entry in entries {
        if let Some(exc) = exclude
            && entry.start == exc.start
            && entry.end == exc.end
        {
            continue;
        }

        if entry.end <= target_start {
            match previous {
                Some(prev) if entry.end > prev.end => previous = Some(entry),
                None => previous = Some(entry),
                _ => {}
            }
        }

        if entry.start >= target_end {
            match next {
                Some(nxt) if entry.start < nxt.start => next = Some(entry),
                None => next = Some(entry),
                _ => {}
            }
        }
    }

    AdjacentEntries {
        previous: previous.cloned(),
        next: next.cloned(),
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

    fn time(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    fn entry(sh: u32, sm: u32, eh: u32, em: u32, desc: &str) -> TimeEntry {
        TimeEntry {
            start: dt(2026, 3, 15, sh, sm),
            end: dt(2026, 3, 15, eh, em),
            description: desc.into(),
            client: "acme".into(),
            project: None,
            activity: None,
            notes: None,
        }
    }

    // snap_to_grid tests
    #[test]
    fn snap_exact_boundary() {
        assert_eq!(
            snap_to_grid(dt(2026, 3, 15, 10, 0), 15),
            dt(2026, 3, 15, 10, 0)
        );
    }

    #[test]
    fn snap_rounds_down() {
        assert_eq!(
            snap_to_grid(dt(2026, 3, 15, 10, 7), 15),
            dt(2026, 3, 15, 10, 0)
        );
    }

    #[test]
    fn snap_rounds_up() {
        assert_eq!(
            snap_to_grid(dt(2026, 3, 15, 10, 8), 15),
            dt(2026, 3, 15, 10, 15)
        );
    }

    #[test]
    fn snap_rolls_to_next_hour() {
        assert_eq!(
            snap_to_grid(dt(2026, 3, 15, 10, 53), 15),
            dt(2026, 3, 15, 11, 0)
        );
    }

    #[test]
    fn snap_zero_returns_unchanged() {
        assert_eq!(
            snap_to_grid(dt(2026, 3, 15, 10, 7), 0),
            dt(2026, 3, 15, 10, 7)
        );
    }

    #[test]
    fn snap_30_minute() {
        assert_eq!(
            snap_to_grid(dt(2026, 3, 15, 10, 14), 30),
            dt(2026, 3, 15, 10, 0)
        );
        assert_eq!(
            snap_to_grid(dt(2026, 3, 15, 10, 16), 30),
            dt(2026, 3, 15, 10, 30)
        );
    }

    #[test]
    fn snap_5_minute() {
        assert_eq!(
            snap_to_grid(dt(2026, 3, 15, 10, 22), 5),
            dt(2026, 3, 15, 10, 20)
        );
        assert_eq!(
            snap_to_grid(dt(2026, 3, 15, 10, 23), 5),
            dt(2026, 3, 15, 10, 25)
        );
    }

    // format_duration tests
    #[test]
    fn format_zero() {
        assert_eq!(format_duration(0), "0m");
    }

    #[test]
    fn format_minutes_only() {
        assert_eq!(format_duration(30), "30m");
    }

    #[test]
    fn format_hours_only() {
        assert_eq!(format_duration(120), "2h");
    }

    #[test]
    fn format_hours_and_minutes() {
        assert_eq!(format_duration(90), "1h 30m");
    }

    #[test]
    fn format_negative() {
        assert_eq!(format_duration(-45), "-45m");
    }

    // parse_duration tests
    #[test]
    fn parse_hours_and_minutes() {
        assert_eq!(parse_duration("1h 30m"), Some(90));
        assert_eq!(parse_duration("1h30m"), Some(90));
    }

    #[test]
    fn parse_minutes_only() {
        assert_eq!(parse_duration("90m"), Some(90));
        assert_eq!(parse_duration("45m"), Some(45));
    }

    #[test]
    fn parse_hours_only() {
        assert_eq!(parse_duration("2h"), Some(120));
    }

    #[test]
    fn parse_decimal_hours() {
        assert_eq!(parse_duration("1.5h"), Some(90));
        assert_eq!(parse_duration("1,5h"), Some(90));
    }

    #[test]
    fn parse_colon_notation() {
        assert_eq!(parse_duration("1:30"), Some(90));
        assert_eq!(parse_duration("0:45"), Some(45));
    }

    #[test]
    fn parse_invalid() {
        assert_eq!(parse_duration(""), None);
        assert_eq!(parse_duration("abc"), None);
    }

    // format_time tests
    #[test]
    fn format_24h() {
        assert_eq!(format_time(time(9, 30), true), "09:30");
        assert_eq!(format_time(time(14, 0), true), "14:00");
        assert_eq!(format_time(time(0, 15), true), "00:15");
    }

    #[test]
    fn format_12h() {
        assert_eq!(format_time(time(9, 30), false), "9:30 AM");
        assert_eq!(format_time(time(14, 0), false), "2:00 PM");
        assert_eq!(format_time(time(0, 15), false), "12:15 AM");
        assert_eq!(format_time(time(12, 0), false), "12:00 PM");
    }

    // find_adjacent tests
    #[test]
    fn find_adjacent_basic() {
        let entries = vec![
            entry(9, 0, 10, 0, "first"),
            entry(11, 0, 12, 0, "second"),
            entry(13, 0, 14, 0, "third"),
        ];
        let adj = find_adjacent(
            dt(2026, 3, 15, 10, 30),
            dt(2026, 3, 15, 11, 0),
            &entries,
            None,
        );
        assert_eq!(adj.previous.unwrap().description, "first");
        assert_eq!(adj.next.unwrap().description, "second");
    }

    #[test]
    fn find_adjacent_no_previous() {
        let entries = vec![entry(11, 0, 12, 0, "only")];
        let adj = find_adjacent(
            dt(2026, 3, 15, 9, 0),
            dt(2026, 3, 15, 10, 0),
            &entries,
            None,
        );
        assert!(adj.previous.is_none());
        assert_eq!(adj.next.unwrap().description, "only");
    }

    #[test]
    fn find_adjacent_no_next() {
        let entries = vec![entry(9, 0, 10, 0, "only")];
        let adj = find_adjacent(
            dt(2026, 3, 15, 14, 0),
            dt(2026, 3, 15, 15, 0),
            &entries,
            None,
        );
        assert_eq!(adj.previous.unwrap().description, "only");
        assert!(adj.next.is_none());
    }

    // compute_available_slots tests

    #[test]
    fn available_slots_no_entries() {
        let slots =
            compute_available_slots(dt(2026, 3, 15, 9, 0), dt(2026, 3, 15, 17, 0), &[], None);
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].start, dt(2026, 3, 15, 9, 0));
        assert_eq!(slots[0].end, dt(2026, 3, 15, 17, 0));
        assert_eq!(slots[0].duration_minutes, 480);
    }

    #[test]
    fn available_slots_fully_occupied() {
        let entries = vec![entry(9, 0, 17, 0, "all day")];
        let slots = compute_available_slots(
            dt(2026, 3, 15, 9, 0),
            dt(2026, 3, 15, 17, 0),
            &entries,
            None,
        );
        assert!(slots.is_empty());
    }

    #[test]
    fn available_slots_gaps_between_entries() {
        let entries = vec![
            entry(9, 0, 10, 0, "first"),
            entry(11, 0, 12, 0, "second"),
            entry(14, 0, 15, 0, "third"),
        ];
        let slots = compute_available_slots(
            dt(2026, 3, 15, 9, 0),
            dt(2026, 3, 15, 17, 0),
            &entries,
            None,
        );
        assert_eq!(slots.len(), 3);
        // Gap 1: 10:00 - 11:00
        assert_eq!(slots[0].start, dt(2026, 3, 15, 10, 0));
        assert_eq!(slots[0].end, dt(2026, 3, 15, 11, 0));
        assert_eq!(slots[0].duration_minutes, 60);
        // Gap 2: 12:00 - 14:00
        assert_eq!(slots[1].start, dt(2026, 3, 15, 12, 0));
        assert_eq!(slots[1].end, dt(2026, 3, 15, 14, 0));
        assert_eq!(slots[1].duration_minutes, 120);
        // Gap 3: 15:00 - 17:00
        assert_eq!(slots[2].start, dt(2026, 3, 15, 15, 0));
        assert_eq!(slots[2].end, dt(2026, 3, 15, 17, 0));
        assert_eq!(slots[2].duration_minutes, 120);
    }

    #[test]
    fn available_slots_back_to_back_entries() {
        let entries = vec![
            entry(9, 0, 10, 0, "first"),
            entry(10, 0, 11, 0, "second"),
            entry(11, 0, 12, 0, "third"),
        ];
        let slots = compute_available_slots(
            dt(2026, 3, 15, 9, 0),
            dt(2026, 3, 15, 12, 0),
            &entries,
            None,
        );
        assert!(slots.is_empty());
    }

    #[test]
    fn available_slots_gap_before_first_entry() {
        let entries = vec![entry(10, 0, 11, 0, "late start")];
        let slots = compute_available_slots(
            dt(2026, 3, 15, 9, 0),
            dt(2026, 3, 15, 12, 0),
            &entries,
            None,
        );
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].start, dt(2026, 3, 15, 9, 0));
        assert_eq!(slots[0].end, dt(2026, 3, 15, 10, 0));
        assert_eq!(slots[1].start, dt(2026, 3, 15, 11, 0));
        assert_eq!(slots[1].end, dt(2026, 3, 15, 12, 0));
    }

    #[test]
    fn available_slots_min_duration_filter() {
        let entries = vec![
            entry(9, 0, 9, 45, "first"),
            // 15 min gap
            entry(10, 0, 11, 0, "second"),
            // 60 min gap
            entry(12, 0, 13, 0, "third"),
        ];
        let slots = compute_available_slots(
            dt(2026, 3, 15, 9, 0),
            dt(2026, 3, 15, 14, 0),
            &entries,
            Some(30), // only slots >= 30 min
        );
        // Should exclude the 15-min gap (9:45-10:00)
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].start, dt(2026, 3, 15, 11, 0));
        assert_eq!(slots[0].duration_minutes, 60);
        assert_eq!(slots[1].start, dt(2026, 3, 15, 13, 0));
        assert_eq!(slots[1].duration_minutes, 60);
    }

    #[test]
    fn available_slots_entry_partially_before_range() {
        // Entry starts at 8:00 but range starts at 9:00
        let entries = vec![TimeEntry {
            start: dt(2026, 3, 15, 8, 0),
            end: dt(2026, 3, 15, 10, 0),
            description: "early".into(),
            client: "acme".into(),
            project: None,
            activity: None,
            notes: None,
        }];
        let slots = compute_available_slots(
            dt(2026, 3, 15, 9, 0),
            dt(2026, 3, 15, 12, 0),
            &entries,
            None,
        );
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].start, dt(2026, 3, 15, 10, 0));
        assert_eq!(slots[0].end, dt(2026, 3, 15, 12, 0));
    }

    #[test]
    fn available_slots_entry_partially_after_range() {
        // Entry ends at 18:00 but range ends at 17:00
        let entries = vec![TimeEntry {
            start: dt(2026, 3, 15, 16, 0),
            end: dt(2026, 3, 15, 18, 0),
            description: "late".into(),
            client: "acme".into(),
            project: None,
            activity: None,
            notes: None,
        }];
        let slots = compute_available_slots(
            dt(2026, 3, 15, 9, 0),
            dt(2026, 3, 15, 17, 0),
            &entries,
            None,
        );
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].start, dt(2026, 3, 15, 9, 0));
        assert_eq!(slots[0].end, dt(2026, 3, 15, 16, 0));
    }

    #[test]
    fn available_slots_overnight_span() {
        // Range spans midnight
        let entries = vec![TimeEntry {
            start: dt(2026, 3, 15, 22, 0),
            end: dt(2026, 3, 16, 1, 0),
            description: "night work".into(),
            client: "acme".into(),
            project: None,
            activity: None,
            notes: None,
        }];
        let slots = compute_available_slots(
            dt(2026, 3, 15, 20, 0),
            dt(2026, 3, 16, 6, 0),
            &entries,
            None,
        );
        assert_eq!(slots.len(), 2);
        // Before: 20:00 - 22:00
        assert_eq!(slots[0].start, dt(2026, 3, 15, 20, 0));
        assert_eq!(slots[0].end, dt(2026, 3, 15, 22, 0));
        assert_eq!(slots[0].duration_minutes, 120);
        // After: 01:00 - 06:00
        assert_eq!(slots[1].start, dt(2026, 3, 16, 1, 0));
        assert_eq!(slots[1].end, dt(2026, 3, 16, 6, 0));
        assert_eq!(slots[1].duration_minutes, 300);
    }

    #[test]
    fn available_slots_entries_outside_range_ignored() {
        let entries = vec![
            entry(7, 0, 8, 0, "too early"),
            entry(10, 0, 11, 0, "in range"),
            entry(18, 0, 19, 0, "too late"),
        ];
        let slots = compute_available_slots(
            dt(2026, 3, 15, 9, 0),
            dt(2026, 3, 15, 12, 0),
            &entries,
            None,
        );
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].start, dt(2026, 3, 15, 9, 0));
        assert_eq!(slots[0].end, dt(2026, 3, 15, 10, 0));
        assert_eq!(slots[1].start, dt(2026, 3, 15, 11, 0));
        assert_eq!(slots[1].end, dt(2026, 3, 15, 12, 0));
    }

    #[test]
    fn available_slots_overlapping_entries() {
        // Two entries that overlap each other (shouldn't happen in practice,
        // but the algorithm handles it gracefully)
        let entries = vec![
            entry(9, 0, 10, 30, "first"),
            entry(10, 0, 11, 0, "overlaps first"),
        ];
        let slots = compute_available_slots(
            dt(2026, 3, 15, 9, 0),
            dt(2026, 3, 15, 12, 0),
            &entries,
            None,
        );
        // Second entry extends occupied range to 11:00
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].start, dt(2026, 3, 15, 11, 0));
        assert_eq!(slots[0].end, dt(2026, 3, 15, 12, 0));
    }

    #[test]
    fn find_adjacent_with_exclude() {
        let self_entry = entry(10, 0, 11, 0, "self");
        let entries = vec![
            entry(9, 0, 10, 0, "before"),
            self_entry.clone(),
            entry(11, 0, 12, 0, "after"),
        ];
        let adj = find_adjacent(
            dt(2026, 3, 15, 10, 0),
            dt(2026, 3, 15, 11, 0),
            &entries,
            Some(&self_entry),
        );
        assert_eq!(adj.previous.unwrap().description, "before");
        assert_eq!(adj.next.unwrap().description, "after");
    }
}
