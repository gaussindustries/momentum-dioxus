//! Calendar view state: which span is on screen and how navigation moves it.
//!
//! Each view knows the inclusive `[from, to]` date window it covers, which is
//! exactly what `TimeStore::occurrences_in` wants.

use chrono::{Datelike, Duration, Months, NaiveDate};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Season {
    Winter,
    Spring,
    Summer,
    Fall,
}

impl Season {
    /// Meteorological seasons (Northern Hemisphere). Winter is anchored to its
    /// December for windowing purposes.
    pub fn of_month(month: u32) -> Season {
        match month {
            12 | 1 | 2 => Season::Winter,
            3 | 4 | 5 => Season::Spring,
            6 | 7 | 8 => Season::Summer,
            _ => Season::Fall,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Season::Winter => "Winter",
            Season::Spring => "Spring",
            Season::Summer => "Summer",
            Season::Fall => "Fall",
        }
    }
    /// First month of the season (Winter -> December).
    pub fn first_month(self) -> u32 {
        match self {
            Season::Winter => 12,
            Season::Spring => 3,
            Season::Summer => 6,
            Season::Fall => 9,
        }
    }
    pub fn next(self) -> Season {
        match self {
            Season::Winter => Season::Spring,
            Season::Spring => Season::Summer,
            Season::Summer => Season::Fall,
            Season::Fall => Season::Winter,
        }
    }
    pub fn prev(self) -> Season {
        match self {
            Season::Winter => Season::Fall,
            Season::Spring => Season::Winter,
            Season::Summer => Season::Spring,
            Season::Fall => Season::Summer,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum CalendarView {
    Decade {
        start_year: i32,
    },
    Year {
        year: i32,
    },
    Season {
        year: i32,
        season: Season,
    },
    Month {
        year: i32,
        month: u32,
    },
    Week {
        anchor: NaiveDate,
    },
    Day {
        date: NaiveDate,
    },
    /// User-picked arbitrary span, rendered as an agenda.
    Range {
        from: NaiveDate,
        to: NaiveDate,
    },
}

impl CalendarView {
    pub fn kind_label(&self) -> &'static str {
        match self {
            CalendarView::Decade { .. } => "Decade",
            CalendarView::Year { .. } => "Year",
            CalendarView::Season { .. } => "Season",
            CalendarView::Month { .. } => "Month",
            CalendarView::Week { .. } => "Week",
            CalendarView::Day { .. } => "Day",
            CalendarView::Range { .. } => "Range",
        }
    }

    /// Inclusive date window this view displays.
    pub fn window(&self) -> (NaiveDate, NaiveDate) {
        match self {
            CalendarView::Decade { start_year } => {
                (ymd(*start_year, 1, 1), ymd(start_year + 9, 12, 31))
            }
            CalendarView::Year { year } => (ymd(*year, 1, 1), ymd(*year, 12, 31)),
            CalendarView::Season { year, season } => {
                let first = first_of_month(*year, season.first_month());
                let start = if season.first_month() == 12 {
                    // Winter spans Dec of `year` .. Feb of `year+1`.
                    first
                } else {
                    first
                };
                let end = add_months(start, 3) - Duration::days(1);
                (start, end)
            }
            CalendarView::Month { year, month } => {
                let start = first_of_month(*year, *month);
                let end = add_months(start, 1) - Duration::days(1);
                (start, end)
            }
            CalendarView::Week { anchor } => {
                let start = week_start(*anchor);
                (start, start + Duration::days(6))
            }
            CalendarView::Day { date } => (*date, *date),
            CalendarView::Range { from, to } => {
                if from <= to {
                    (*from, *to)
                } else {
                    (*to, *from)
                }
            }
        }
    }

    /// A human-readable title for the toolbar.
    pub fn title(&self) -> String {
        match self {
            CalendarView::Decade { start_year } => {
                format!("{}–{}", start_year, start_year + 9)
            }
            CalendarView::Year { year } => year.to_string(),
            CalendarView::Season { year, season } => format!("{} {}", season.label(), year),
            CalendarView::Month { year, month } => {
                format!("{} {}", month_name(*month), year)
            }
            CalendarView::Week { anchor } => {
                let s = week_start(*anchor);
                let e = s + Duration::days(6);
                format!("{} – {}", s.format("%b %-d"), e.format("%b %-d, %Y"))
            }
            CalendarView::Day { date } => date.format("%A, %B %-d, %Y").to_string(),
            CalendarView::Range { from, to } => {
                let (a, b) = if from <= to { (from, to) } else { (to, from) };
                format!("{} – {}", a.format("%b %-d, %Y"), b.format("%b %-d, %Y"))
            }
        }
    }

    /// Step one unit backward.
    pub fn prev(&self) -> CalendarView {
        match self {
            CalendarView::Decade { start_year } => CalendarView::Decade {
                start_year: start_year - 10,
            },
            CalendarView::Year { year } => CalendarView::Year { year: year - 1 },
            CalendarView::Season { year, season } => {
                let p = season.prev();
                let y = if *season == Season::Winter {
                    year - 1
                } else {
                    *year
                };
                // Going back from Winter wraps to previous year's Fall.
                let y = if p == Season::Fall && *season == Season::Winter {
                    *year - 1
                } else if *season == Season::Winter {
                    *year
                } else {
                    y
                };
                CalendarView::Season { year: y, season: p }
            }
            CalendarView::Month { year, month } => {
                let d = first_of_month(*year, *month) - Duration::days(1);
                CalendarView::Month {
                    year: d.year(),
                    month: d.month(),
                }
            }
            CalendarView::Week { anchor } => CalendarView::Week {
                anchor: *anchor - Duration::days(7),
            },
            CalendarView::Day { date } => CalendarView::Day {
                date: *date - Duration::days(1),
            },
            CalendarView::Range { from, to } => {
                let span = (*to - *from).num_days().abs() + 1;
                CalendarView::Range {
                    from: *from - Duration::days(span),
                    to: *to - Duration::days(span),
                }
            }
        }
    }

    /// Step one unit forward.
    pub fn next(&self) -> CalendarView {
        match self {
            CalendarView::Decade { start_year } => CalendarView::Decade {
                start_year: start_year + 10,
            },
            CalendarView::Year { year } => CalendarView::Year { year: year + 1 },
            CalendarView::Season { year, season } => {
                let n = season.next();
                let y = if n == Season::Winter { year + 1 } else { *year };
                CalendarView::Season { year: y, season: n }
            }
            CalendarView::Month { year, month } => {
                let d = add_months(first_of_month(*year, *month), 1);
                CalendarView::Month {
                    year: d.year(),
                    month: d.month(),
                }
            }
            CalendarView::Week { anchor } => CalendarView::Week {
                anchor: *anchor + Duration::days(7),
            },
            CalendarView::Day { date } => CalendarView::Day {
                date: *date + Duration::days(1),
            },
            CalendarView::Range { from, to } => {
                let span = (*to - *from).num_days().abs() + 1;
                CalendarView::Range {
                    from: *from + Duration::days(span),
                    to: *to + Duration::days(span),
                }
            }
        }
    }

    /// Sensible default views built around "today".
    pub fn today_month(today: NaiveDate) -> Self {
        CalendarView::Month {
            year: today.year(),
            month: today.month(),
        }
    }
    pub fn today_week(today: NaiveDate) -> Self {
        CalendarView::Week { anchor: today }
    }
    pub fn today_day(today: NaiveDate) -> Self {
        CalendarView::Day { date: today }
    }
    pub fn this_year(today: NaiveDate) -> Self {
        CalendarView::Year { year: today.year() }
    }
    pub fn this_decade(today: NaiveDate) -> Self {
        CalendarView::Decade {
            start_year: today.year() - today.year().rem_euclid(10),
        }
    }
    pub fn this_season(today: NaiveDate) -> Self {
        let season = Season::of_month(today.month());
        // Winter in Jan/Feb belongs to the prior December.
        let year = if season == Season::Winter && today.month() < 12 {
            today.year() - 1
        } else {
            today.year()
        };
        CalendarView::Season { year, season }
    }
}

// --- small date helpers ----------------------------------------------------

pub fn ymd(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap_or_else(|| NaiveDate::from_ymd_opt(y, m, 28).unwrap())
}

pub fn first_of_month(year: i32, month: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, 1).unwrap()
}

pub fn add_months(date: NaiveDate, n: u32) -> NaiveDate {
    date.checked_add_months(Months::new(n)).unwrap_or(date)
}

/// Monday-based start of the week containing `date`.
pub fn week_start(date: NaiveDate) -> NaiveDate {
    date - Duration::days(date.weekday().num_days_from_monday() as i64)
}

pub fn month_name(month: u32) -> &'static str {
    [
        "",
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ]
    .get(month as usize)
    .copied()
    .unwrap_or("")
}

pub fn month_abbr(month: u32) -> &'static str {
    [
        "", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ]
    .get(month as usize)
    .copied()
    .unwrap_or("")
}
