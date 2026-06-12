//! Scheduler data model (the `time` module's event + recurrence engine).
//!
//! This module is deliberately independent of `dioxus-primitives` so that the
//! persisted JSON format never breaks when the (pre-1.0) primitives crate
//! changes its `CalendarDate` shape. Everything here is plain `chrono` + serde.

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stable identity for an event. v4 UUID — generated in the store on insert, so
/// there's no counter to persist or keep in sync.
pub type EventId = Uuid;

/// Which sub-app created/owns an event. Drives the color shown on the calendar
/// and lets a view filter down to one source.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum EventSource {
    Manual,
    Health,
    JaxBrain,
    FinCalc,
}

impl Default for EventSource {
    fn default() -> Self {
        EventSource::Manual
    }
}

impl EventSource {
    /// CSS custom property used for the chip color. Define these in style.css.
    pub fn color_var(self) -> &'static str {
        match self {
            EventSource::Manual => "var(--sched-manual, #6b7280)",
            EventSource::Health => "var(--sched-health, #16a34a)",
            EventSource::JaxBrain => "var(--sched-jax, #7c3aed)",
            EventSource::FinCalc => "var(--sched-fin, #2563eb)",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            EventSource::Manual => "General",
            EventSource::Health => "Health",
            EventSource::JaxBrain => "Jax Brain",
            EventSource::FinCalc => "Finance",
        }
    }

    pub const ALL: [EventSource; 4] = [
        EventSource::Manual,
        EventSource::Health,
        EventSource::JaxBrain,
        EventSource::FinCalc,
    ];
}

/// When an event happens: either an all-day marker on a date, or a timed block.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum When {
    AllDay {
        date: NaiveDate,
    },
    Timed {
        start: NaiveDateTime,
        end: NaiveDateTime,
    },
}

impl When {
    /// The anchor date a recurrence is calculated from.
    pub fn anchor_date(&self) -> NaiveDate {
        match self {
            When::AllDay { date } => *date,
            When::Timed { start, .. } => start.date(),
        }
    }

    pub fn is_all_day(&self) -> bool {
        matches!(self, When::AllDay { .. })
    }

    /// Duration of a timed block; zero for all-day.
    pub fn duration(&self) -> Duration {
        match self {
            When::AllDay { .. } => Duration::zero(),
            When::Timed { start, end } => *end - *start,
        }
    }

    pub fn start_time(&self) -> NaiveTime {
        match self {
            When::AllDay { .. } => NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            When::Timed { start, .. } => start.time(),
        }
    }
}

/// Recurrence frequency unit.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Freq {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl Default for Freq {
    fn default() -> Self {
        Freq::Weekly
    }
}

impl Freq {
    pub fn label(self) -> &'static str {
        match self {
            Freq::Daily => "day",
            Freq::Weekly => "week",
            Freq::Monthly => "month",
            Freq::Yearly => "year",
        }
    }
    pub const ALL: [Freq; 4] = [Freq::Daily, Freq::Weekly, Freq::Monthly, Freq::Yearly];
}

/// A practical RFC-5545-lite recurrence rule.
///
/// Weekdays are stored as 0=Mon .. 6=Sun (matching
/// `chrono::Weekday::num_days_from_monday`) to keep the JSON stable and
/// serde-trivial.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Recurrence {
    pub freq: Freq,
    /// Every `interval` units (e.g. interval=2 + Weekly = fortnightly).
    pub interval: u32,
    /// Only meaningful for `Weekly`. Empty => use the anchor's own weekday.
    #[serde(default)]
    pub by_weekday: Vec<u8>,
    /// Stop after this many occurrences (counted from the series start).
    #[serde(default)]
    pub count: Option<u32>,
    /// Stop on/after this date (inclusive).
    #[serde(default)]
    pub until: Option<NaiveDate>,
}

impl Default for Recurrence {
    fn default() -> Self {
        Recurrence {
            freq: Freq::Weekly,
            interval: 1,
            by_weekday: Vec::new(),
            count: None,
            until: None,
        }
    }
}

impl Recurrence {
    pub fn human(&self) -> String {
        let every = if self.interval <= 1 {
            format!("every {}", self.freq.label())
        } else {
            format!("every {} {}s", self.interval, self.freq.label())
        };
        let ending = match (self.count, self.until) {
            (Some(c), _) => format!(", {c} times"),
            (_, Some(u)) => format!(", until {u}"),
            _ => String::new(),
        };
        format!("{every}{ending}")
    }
}

/// A stored event. One `Event` can materialize into many `Occurrence`s when it
/// recurs.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub title: String,
    #[serde(default)]
    pub notes: String,
    pub when: When,
    #[serde(default)]
    pub source: EventSource,
    #[serde(default)]
    pub recurrence: Option<Recurrence>,
    /// Optional back-reference into the owning sub-app (e.g. a JaxBrain node id,
    /// a FinCalc scenario key). Lets a click on the chip deep-link back.
    #[serde(default)]
    pub link: Option<String>,
}

/// A concrete, materialized instance of an event on a specific day. This is what
/// the calendar views actually render. Occurrences are transient and never
/// persisted.
#[derive(Clone, PartialEq, Debug)]
pub struct Occurrence {
    pub event_id: EventId,
    pub title: String,
    pub source: EventSource,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub all_day: bool,
    pub link: Option<String>,
}

impl Occurrence {
    pub fn date(&self) -> NaiveDate {
        self.start.date()
    }
}

impl Event {
    /// Expand this event into every occurrence whose *date* falls within
    /// `[from, to]` inclusive.
    pub fn occurrences(&self, from: NaiveDate, to: NaiveDate) -> Vec<Occurrence> {
        let anchor = self.when.anchor_date();
        let dur = self.when.duration();
        let all_day = self.when.is_all_day();
        let tod = self.when.start_time();

        let mk = |d: NaiveDate| {
            let start = d.and_time(tod);
            let end = if all_day { start } else { start + dur };
            Occurrence {
                event_id: self.id,
                title: self.title.clone(),
                source: self.source,
                start,
                end,
                all_day,
                link: self.link.clone(),
            }
        };

        match &self.recurrence {
            None => {
                if anchor >= from && anchor <= to {
                    vec![mk(anchor)]
                } else {
                    Vec::new()
                }
            }
            Some(rec) => expand_dates(anchor, rec, from, to)
                .into_iter()
                .map(mk)
                .collect(),
        }
    }
}

const GUARD: u32 = 200_000;

/// Produce the set of dates a recurrence lands on within `[win_from, win_to]`.
/// `count` is honored against the *whole* series (from `base`), not the window.
fn expand_dates(
    base: NaiveDate,
    rec: &Recurrence,
    win_from: NaiveDate,
    win_to: NaiveDate,
) -> Vec<NaiveDate> {
    let mut out = Vec::new();
    let interval = rec.interval.max(1) as i64;
    let mut produced: u32 = 0;
    let mut guard: u32 = 0;

    let count_ok = |produced: u32| rec.count.map_or(true, |c| produced < c);
    let until_ok = |d: NaiveDate| rec.until.map_or(true, |u| d <= u);

    match rec.freq {
        Freq::Daily => {
            let mut d = base;
            while d <= win_to && count_ok(produced) && until_ok(d) {
                if d >= win_from {
                    out.push(d);
                }
                produced += 1;
                d += Duration::days(interval);
                guard += 1;
                if guard > GUARD {
                    break;
                }
            }
        }
        Freq::Weekly => {
            let weekdays: Vec<u8> = if rec.by_weekday.is_empty() {
                vec![base.weekday().num_days_from_monday() as u8]
            } else {
                let mut w = rec.by_weekday.clone();
                w.sort_unstable();
                w.dedup();
                w
            };
            // Anchor to the Monday of the base week.
            let mut week_start =
                base - Duration::days(base.weekday().num_days_from_monday() as i64);
            'weeks: loop {
                for &wd in &weekdays {
                    let d = week_start + Duration::days(wd as i64);
                    if d < base {
                        continue; // series hasn't started on this earlier weekday
                    }
                    if !count_ok(produced) || !until_ok(d) || d > win_to {
                        break 'weeks;
                    }
                    produced += 1;
                    if d >= win_from {
                        out.push(d);
                    }
                }
                week_start += Duration::weeks(interval);
                guard += 1;
                if week_start > win_to || guard > GUARD {
                    break;
                }
            }
        }
        Freq::Monthly => {
            // Monthly/yearly dates are strictly increasing as idx grows, so the
            // first time we pass win_to we're done. add_months returning None
            // (day-of-month doesn't exist that month, e.g. Feb 31) is skipped
            // without counting, per RFC 5545.
            let mut idx: i64 = 0;
            loop {
                if guard > GUARD {
                    break;
                }
                guard += 1;
                let d = match add_months(base, idx * interval) {
                    Some(d) => d,
                    None => {
                        idx += 1;
                        continue;
                    }
                };
                idx += 1;
                if !count_ok(produced) || !until_ok(d) || d > win_to {
                    break;
                }
                produced += 1;
                if d >= win_from {
                    out.push(d);
                }
            }
        }
        Freq::Yearly => {
            let mut idx: i64 = 0;
            loop {
                if guard > GUARD {
                    break;
                }
                guard += 1;
                let d = match add_years(base, idx * interval) {
                    Some(d) => d, // Feb 29 on a non-leap year -> None -> skipped
                    None => {
                        idx += 1;
                        continue;
                    }
                };
                idx += 1;
                if !count_ok(produced) || !until_ok(d) || d > win_to {
                    break;
                }
                produced += 1;
                if d >= win_from {
                    out.push(d);
                }
            }
        }
    }
    out
}

/// Add `months` calendar months, preserving day-of-month. Returns `None` when
/// the target month has no such day (e.g. Jan 31 + 1 month), which is then
/// skipped — matching RFC 5545 behavior.
fn add_months(date: NaiveDate, months: i64) -> Option<NaiveDate> {
    let total = (date.year() as i64) * 12 + (date.month0() as i64) + months;
    let year = total.div_euclid(12) as i32;
    let month0 = total.rem_euclid(12) as u32;
    NaiveDate::from_ymd_opt(year, month0 + 1, date.day())
}

fn add_years(date: NaiveDate, years: i64) -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(date.year() + years as i32, date.month(), date.day())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn weekly_mwf_in_window() {
        let ev = Event {
            id: Uuid::nil(),
            title: "Lift".into(),
            notes: String::new(),
            when: When::AllDay {
                date: d(2026, 1, 5),
            }, // Monday
            source: EventSource::Health,
            recurrence: Some(Recurrence {
                freq: Freq::Weekly,
                interval: 1,
                by_weekday: vec![0, 2, 4], // Mon/Wed/Fri
                count: None,
                until: None,
            }),
            link: None,
        };
        let occ = ev.occurrences(d(2026, 1, 5), d(2026, 1, 11));
        assert_eq!(occ.len(), 3);
    }

    #[test]
    fn monthly_skips_missing_day() {
        let ev = Event {
            id: Uuid::nil(),
            title: "Rent".into(),
            notes: String::new(),
            when: When::AllDay {
                date: d(2026, 1, 31),
            },
            source: EventSource::FinCalc,
            recurrence: Some(Recurrence {
                freq: Freq::Monthly,
                interval: 1,
                by_weekday: vec![],
                count: None,
                until: None,
            }),
            link: None,
        };
        // Feb has no 31st -> skipped; Mar 31 exists.
        let occ = ev.occurrences(d(2026, 2, 1), d(2026, 3, 31));
        assert_eq!(occ.len(), 1);
        assert_eq!(occ[0].date(), d(2026, 3, 31));
    }

    #[test]
    fn count_limits_series() {
        let ev = Event {
            id: Uuid::nil(),
            title: "Standup".into(),
            notes: String::new(),
            when: When::Timed {
                start: d(2026, 1, 1).and_hms_opt(9, 0, 0).unwrap(),
                end: d(2026, 1, 1).and_hms_opt(9, 15, 0).unwrap(),
            },
            source: EventSource::Manual,
            recurrence: Some(Recurrence {
                freq: Freq::Daily,
                interval: 1,
                by_weekday: vec![],
                count: Some(3),
                until: None,
            }),
            link: None,
        };
        let occ = ev.occurrences(d(2026, 1, 1), d(2026, 12, 31));
        assert_eq!(occ.len(), 3);
    }
}
