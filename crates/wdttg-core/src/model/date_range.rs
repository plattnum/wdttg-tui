use chrono::{Datelike, Days, NaiveDate, Weekday};

/// Preset date ranges for quick filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeRangePreset {
    Today,
    Yesterday,
    ThisWeek,
    LastWeek,
    ThisMonth,
    LastMonth,
    Custom,
}

/// A date range for filtering entries and generating reports. Both ends inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

impl DateRange {
    pub fn new(start: NaiveDate, end: NaiveDate) -> Self {
        Self { start, end }
    }

    /// Create a DateRange from a preset relative to a reference date.
    pub fn from_preset(
        preset: TimeRangePreset,
        reference_date: NaiveDate,
        week_start: &str,
    ) -> Self {
        match preset {
            TimeRangePreset::Today => Self::new(reference_date, reference_date),
            TimeRangePreset::Yesterday => {
                let yesterday = reference_date.pred_opt().unwrap();
                Self::new(yesterday, yesterday)
            }
            TimeRangePreset::ThisWeek => Self::week_containing(reference_date, week_start),
            TimeRangePreset::LastWeek => {
                let last_week = reference_date.checked_sub_days(Days::new(7)).unwrap();
                Self::week_containing(last_week, week_start)
            }
            TimeRangePreset::ThisMonth => Self::month_containing(reference_date),
            TimeRangePreset::LastMonth => {
                let first_of_current = reference_date.with_day(1).unwrap();
                let last_of_prev = first_of_current.pred_opt().unwrap();
                Self::month_containing(last_of_prev)
            }
            TimeRangePreset::Custom => Self::new(reference_date, reference_date),
        }
    }

    /// All YYYY-MM month keys this range spans.
    pub fn months_spanned(&self) -> Vec<String> {
        let mut months = Vec::new();
        let mut year = self.start.year();
        let mut month = self.start.month();

        loop {
            months.push(format!("{year:04}-{month:02}"));

            if year == self.end.year() && month == self.end.month() {
                break;
            }

            if month == 12 {
                year += 1;
                month = 1;
            } else {
                month += 1;
            }
        }

        months
    }

    fn week_containing(date: NaiveDate, week_start: &str) -> Self {
        let start_weekday = if week_start == "sunday" {
            Weekday::Sun
        } else {
            Weekday::Mon
        };

        let days_since_start =
            (date.weekday().num_days_from_monday() + 7 - start_weekday.num_days_from_monday()) % 7;

        let week_start_date = date
            .checked_sub_days(Days::new(days_since_start as u64))
            .unwrap();
        let week_end_date = week_start_date.checked_add_days(Days::new(6)).unwrap();

        Self::new(week_start_date, week_end_date)
    }

    fn month_containing(date: NaiveDate) -> Self {
        let first = date.with_day(1).unwrap();
        let last = if date.month() == 12 {
            NaiveDate::from_ymd_opt(date.year() + 1, 1, 1)
                .unwrap()
                .pred_opt()
                .unwrap()
        } else {
            NaiveDate::from_ymd_opt(date.year(), date.month() + 1, 1)
                .unwrap()
                .pred_opt()
                .unwrap()
        };
        Self::new(first, last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn today_preset() {
        let r = DateRange::from_preset(TimeRangePreset::Today, date(2026, 3, 15), "monday");
        assert_eq!(r.start, date(2026, 3, 15));
        assert_eq!(r.end, date(2026, 3, 15));
    }

    #[test]
    fn yesterday_preset() {
        let r = DateRange::from_preset(TimeRangePreset::Yesterday, date(2026, 3, 15), "monday");
        assert_eq!(r.start, date(2026, 3, 14));
        assert_eq!(r.end, date(2026, 3, 14));
    }

    #[test]
    fn yesterday_crosses_year_boundary() {
        let r = DateRange::from_preset(TimeRangePreset::Yesterday, date(2026, 1, 1), "monday");
        assert_eq!(r.start, date(2025, 12, 31));
    }

    #[test]
    fn this_week_monday_start() {
        // 2026-03-15 is a Sunday
        let r = DateRange::from_preset(TimeRangePreset::ThisWeek, date(2026, 3, 15), "monday");
        assert_eq!(r.start, date(2026, 3, 9)); // Monday
        assert_eq!(r.end, date(2026, 3, 15)); // Sunday
    }

    #[test]
    fn this_week_sunday_start() {
        // 2026-03-15 is a Sunday
        let r = DateRange::from_preset(TimeRangePreset::ThisWeek, date(2026, 3, 15), "sunday");
        assert_eq!(r.start, date(2026, 3, 15)); // Sunday
        assert_eq!(r.end, date(2026, 3, 21)); // Saturday
    }

    #[test]
    fn last_week_preset() {
        let r = DateRange::from_preset(TimeRangePreset::LastWeek, date(2026, 3, 15), "monday");
        assert_eq!(r.start, date(2026, 3, 2)); // Previous Monday
        assert_eq!(r.end, date(2026, 3, 8)); // Previous Sunday
    }

    #[test]
    fn this_month_preset() {
        let r = DateRange::from_preset(TimeRangePreset::ThisMonth, date(2026, 3, 15), "monday");
        assert_eq!(r.start, date(2026, 3, 1));
        assert_eq!(r.end, date(2026, 3, 31));
    }

    #[test]
    fn last_month_from_january() {
        let r = DateRange::from_preset(TimeRangePreset::LastMonth, date(2026, 1, 15), "monday");
        assert_eq!(r.start, date(2025, 12, 1));
        assert_eq!(r.end, date(2025, 12, 31));
    }

    #[test]
    fn last_month_february_non_leap() {
        let r = DateRange::from_preset(TimeRangePreset::LastMonth, date(2026, 3, 15), "monday");
        assert_eq!(r.start, date(2026, 2, 1));
        assert_eq!(r.end, date(2026, 2, 28));
    }

    #[test]
    fn months_spanned_single_day() {
        let r = DateRange::new(date(2026, 3, 15), date(2026, 3, 15));
        assert_eq!(r.months_spanned(), vec!["2026-03"]);
    }

    #[test]
    fn months_spanned_single_month() {
        let r = DateRange::new(date(2026, 3, 1), date(2026, 3, 31));
        assert_eq!(r.months_spanned(), vec!["2026-03"]);
    }

    #[test]
    fn months_spanned_two_months() {
        let r = DateRange::new(date(2026, 3, 15), date(2026, 4, 15));
        assert_eq!(r.months_spanned(), vec!["2026-03", "2026-04"]);
    }

    #[test]
    fn months_spanned_crosses_year() {
        let r = DateRange::new(date(2025, 11, 1), date(2026, 2, 28));
        assert_eq!(
            r.months_spanned(),
            vec!["2025-11", "2025-12", "2026-01", "2026-02"]
        );
    }
}
