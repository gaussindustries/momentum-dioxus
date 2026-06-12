//! Momentum time/scheduler UI.
//!
//! Top-level component: [`Time`]. `Time {}` is the full experience (calendar +
//! weekly planner); `Time { overview: true }` is the compact month + agenda for
//! the Overview page.
//!
//! Requires the shared store to be provided above this component — call
//! `use_provide_time()` in your root `App` (see store.rs).

use std::collections::BTreeMap;

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use dioxus::prelude::*;

use super::model::{Event, EventId, EventSource, Freq, Occurrence, Recurrence, When};
use super::store::{use_time, TimeStore};
use super::view::{
    add_months, first_of_month, month_abbr, month_name, week_start, CalendarView, Season,
};

/// Today's local date. Needs chrono's `clock` feature (on by default for the
/// desktop target).
pub fn today_local() -> NaiveDate {
    chrono::Local::now().date_naive()
}

// Planner grid bounds.
const PLAN_START_HOUR: u32 = 6;
const PLAN_END_HOUR: u32 = 23; // exclusive end of the last start slot
const SLOTS_PER_HOUR: u32 = 2; // 30-minute granularity
const PLAN_SLOTS: u32 = (PLAN_END_HOUR - PLAN_START_HOUR) * SLOTS_PER_HOUR;

const WEEKDAY_ABBR: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

// ---------------------------------------------------------------------------
// Editor targets
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
pub enum EditTarget {
    New(Prefill),
    Edit(Event),
}

#[derive(Clone, PartialEq)]
pub struct Prefill {
    pub source: EventSource,
    pub date: NaiveDate,
    pub all_day: bool,
    pub start: Option<NaiveTime>,
    pub end: Option<NaiveTime>,
    /// If set, prefill a weekly recurrence on this weekday (0=Mon).
    pub weekly_on: Option<u8>,
}

impl Prefill {
    fn blank(date: NaiveDate) -> Self {
        Prefill {
            source: EventSource::Manual,
            date,
            all_day: false,
            start: NaiveTime::from_hms_opt(9, 0, 0),
            end: NaiveTime::from_hms_opt(10, 0, 0),
            weekly_on: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Top-level component
// ---------------------------------------------------------------------------

#[component]
pub fn Time(#[props(default)] overview: bool) -> Element {
    let store = use_time();
    let today = today_local();

    let mut current = use_signal(|| CalendarView::today_month(today));
    // The active top-level mode: the calendar, or the weekly planner.
    let mut planner_mode = use_signal(|| false);
    // Editor modal target (None = closed).
    let editing = use_signal(|| Option::<EditTarget>::None);

    let open_editor = use_callback(move |t: EditTarget| {
        let mut editing = editing;
        editing.set(Some(t));
    });

    // Compact overview: month grid + agenda, no chrome.
    if overview {
        return rsx! {
            document::Link { rel: "stylesheet", href: asset!("./style.css") }
            div { class: "sched sched-overview",
                div { class: "sched-overview-head",
                    h2 { "{current.read().title()}" }
                    div { class: "sched-nav",
                        button { class: "sched-btn", onclick: move |_| { let v = current.read().prev(); current.set(v); }, "‹" }
                        button { class: "sched-btn", onclick: move |_| current.set(CalendarView::today_month(today)), "Today" }
                        button { class: "sched-btn", onclick: move |_| { let v = current.read().next(); current.set(v); }, "›" }
                    }
                }
                MonthGrid { store, today, view: current, open_editor }
                AgendaStrip { store, today }
            }
            if let Some(t) = editing.read().clone() {
                EventEditor { store, target: t, editing }
            }
        };
    }

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("./style.css") }
        div { class: "sched",
            // ---- Toolbar -------------------------------------------------
            div { class: "sched-toolbar",
                div { class: "sched-modes",
                    button {
                        class: if !*planner_mode.read() { "sched-mode active" } else { "sched-mode" },
                        onclick: move |_| planner_mode.set(false),
                        "Calendar"
                    }
                    button {
                        class: if *planner_mode.read() { "sched-mode active" } else { "sched-mode" },
                        onclick: move |_| planner_mode.set(true),
                        "Weekly planner"
                    }
                }
                div { class: "sched-spacer" }
                button {
                    class: "sched-btn sched-primary",
                    onclick: move |_| open_editor.call(EditTarget::New(Prefill::blank(today))),
                    "+ New event"
                }
            }

            if *planner_mode.read() {
                WeeklyPlanner { store, today, open_editor }
            } else {
                // ---- View switcher --------------------------------------
                ViewSwitcher { current, today }

                // ---- Nav + title ----------------------------------------
                div { class: "sched-header",
                    button { class: "sched-btn", onclick: move |_| { let v = current.read().prev(); current.set(v); }, "‹ Prev" }
                    h2 { class: "sched-title", "{current.read().title()}" }
                    button { class: "sched-btn", onclick: move |_| { let v = current.read().next(); current.set(v); }, "Next ›" }
                }

                // ---- Range pickers (only for Range view) ----------------
                if matches!(*current.read(), CalendarView::Range { .. }) {
                    RangeControls { current }
                }

                // ---- The view body --------------------------------------
                ViewBody { store, today, view: current, open_editor }
            }

            // Color legend
            div { class: "sched-legend",
                for s in EventSource::ALL {
                    span { class: "sched-legend-item",
                        span { class: "sched-dot", style: "background:{s.color_var()}" }
                        "{s.label()}"
                    }
                }
            }
        }

        if let Some(t) = editing.read().clone() {
            EventEditor { store, target: t, editing }
        }
    }
}

// ---------------------------------------------------------------------------
// View switcher (segmented control over view kinds)
// ---------------------------------------------------------------------------

#[component]
fn ViewSwitcher(current: Signal<CalendarView>, today: NaiveDate) -> Element {
    let active = current.read().kind_label();
    let make = move |label: &'static str, builder: fn(NaiveDate) -> CalendarView| {
        rsx! {
            button {
                key: "{label}",
                class: if active == label { "sched-seg active" } else { "sched-seg" },
                onclick: move |_| current.set(builder(today)),
                "{label}"
            }
        }
    };
    rsx! {
        div { class: "sched-seg-row",
            {make("Decade", CalendarView::this_decade)}
            {make("Year", CalendarView::this_year)}
            {make("Season", CalendarView::this_season)}
            {make("Month", CalendarView::today_month)}
            {make("Week", CalendarView::today_week)}
            {make("Day", CalendarView::today_day)}
            button {
                class: if active == "Range" { "sched-seg active" } else { "sched-seg" },
                onclick: move |_| current.set(CalendarView::Range { from: today, to: today + Duration::days(13) }),
                "Range"
            }
        }
    }
}

#[component]
fn RangeControls(current: Signal<CalendarView>) -> Element {
    let (from, to) = match &*current.read() {
        CalendarView::Range { from, to } => (*from, *to),
        _ => (today_local(), today_local()),
    };
    rsx! {
        div { class: "sched-range-controls",
            label { "From "
                input {
                    r#type: "date",
                    value: "{from.format(\"%Y-%m-%d\")}",
                    onchange: move |e| {
                        if let Ok(d) = NaiveDate::parse_from_str(&e.value(), "%Y-%m-%d") {
                            let to = match &*current.read() { CalendarView::Range { to, .. } => *to, _ => d };
                            current.set(CalendarView::Range { from: d, to });
                        }
                    }
                }
            }
            label { "To "
                input {
                    r#type: "date",
                    value: "{to.format(\"%Y-%m-%d\")}",
                    onchange: move |e| {
                        if let Ok(d) = NaiveDate::parse_from_str(&e.value(), "%Y-%m-%d") {
                            let from = match &*current.read() { CalendarView::Range { from, .. } => *from, _ => d };
                            current.set(CalendarView::Range { from, to: d });
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// View body dispatch
// ---------------------------------------------------------------------------

#[component]
fn ViewBody(
    store: TimeStore,
    today: NaiveDate,
    view: Signal<CalendarView>,
    open_editor: Callback<EditTarget>,
) -> Element {
    let v = view.read().clone();
    match v {
        CalendarView::Decade { .. } => rsx! { DecadeGrid { view, today } },
        CalendarView::Year { .. } => rsx! { YearGrid { store, view, today } },
        CalendarView::Season { .. } => rsx! { SeasonGrid { store, view, today } },
        CalendarView::Month { .. } => rsx! { MonthGrid { store, today, view, open_editor } },
        CalendarView::Week { .. } => rsx! { WeekGrid { store, today, view, open_editor } },
        CalendarView::Day { .. } => rsx! { DayView { store, today, view, open_editor } },
        CalendarView::Range { .. } => rsx! { AgendaView { store, view, open_editor } },
    }
}

// ---------------------------------------------------------------------------
// Shared helper: group occurrences by date
// ---------------------------------------------------------------------------

fn grouped(
    store: &TimeStore,
    from: NaiveDate,
    to: NaiveDate,
) -> BTreeMap<NaiveDate, Vec<Occurrence>> {
    let mut map: BTreeMap<NaiveDate, Vec<Occurrence>> = BTreeMap::new();
    for occ in store.occurrences_in(from, to) {
        map.entry(occ.date()).or_default().push(occ);
    }
    map
}

fn time_label(t: NaiveTime) -> String {
    if t.minute() == 0 {
        t.format("%-I%P").to_string()
    } else {
        t.format("%-I:%M%P").to_string()
    }
}

// ---------------------------------------------------------------------------
// Month grid
// ---------------------------------------------------------------------------

#[component]
fn MonthGrid(
    store: TimeStore,
    today: NaiveDate,
    view: Signal<CalendarView>,
    open_editor: Callback<EditTarget>,
) -> Element {
    let (year, month) = match &*view.read() {
        CalendarView::Month { year, month } => (*year, *month),
        _ => (today.year(), today.month()),
    };
    let first = first_of_month(year, month);
    let grid_start = week_start(first);
    let by_date = grouped(&store, grid_start, grid_start + Duration::days(41));

    rsx! {
        div { class: "sched-month",
            div { class: "sched-weekhead",
                for wd in WEEKDAY_ABBR { div { class: "sched-weekhead-cell", "{wd}" } }
            }
            div { class: "sched-month-grid",
                for i in 0..42i64 {
                    {
                        let date = grid_start + Duration::days(i);
                        let in_month = date.month() == month;
                        let is_today = date == today;
                        let occs = by_date.get(&date).cloned().unwrap_or_default();
                        let shown = occs.iter().take(3).cloned().collect::<Vec<_>>();
                        let extra = occs.len().saturating_sub(3);
                        rsx! {
                            div {
                                key: "{date}",
                                class: if in_month { if is_today { "sched-day in today" } else { "sched-day in" } } else { "sched-day out" },
                                ondoubleclick: move |_| open_editor.call(EditTarget::New(Prefill::blank(date))),
                                onclick: move |_| view.set(CalendarView::Day { date }),
                                div { class: "sched-daynum", "{date.day()}" }
                                div { class: "sched-chips",
                                    for o in shown {
                                        div {
                                            key: "{o.event_id}-{o.start}",
                                            class: "sched-chip",
                                            style: "border-left-color:{o.source.color_var()}",
                                            title: "{o.title}",
                                            if !o.all_day {
                                                span { class: "sched-chip-time", "{time_label(o.start.time())} " }
                                            }
                                            "{o.title}"
                                        }
                                    }
                                    if extra > 0 { div { class: "sched-more", "+{extra} more" } }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Week grid (timed, with hour rail)
// ---------------------------------------------------------------------------

#[component]
fn WeekGrid(
    store: TimeStore,
    today: NaiveDate,
    view: Signal<CalendarView>,
    open_editor: Callback<EditTarget>,
) -> Element {
    let anchor = match &*view.read() {
        CalendarView::Week { anchor } => *anchor,
        _ => today,
    };
    let start = week_start(anchor);
    let by_date = grouped(&store, start, start + Duration::days(6));

    rsx! {
        div { class: "sched-week",
            div { class: "sched-week-head",
                div { class: "sched-rail-spacer" }
                for i in 0..7i64 {
                    {
                        let d = start + Duration::days(i);
                        rsx! {
                            div {
                                key: "{d}",
                                class: if d == today { "sched-week-daycol head today" } else { "sched-week-daycol head" },
                                onclick: move |_| view.set(CalendarView::Day { date: d }),
                                div { class: "sched-week-dow", "{WEEKDAY_ABBR[i as usize]}" }
                                div { class: "sched-week-dom", "{d.day()}" }
                            }
                        }
                    }
                }
            }
            div { class: "sched-week-body",
                div { class: "sched-rail",
                    for h in 0..24u32 {
                        div { key: "{h}", class: "sched-rail-hour", "{h:02}:00" }
                    }
                }
                for i in 0..7i64 {
                    {
                        let d = start + Duration::days(i);
                        let occs = by_date.get(&d).cloned().unwrap_or_default();
                        rsx! {
                            div {
                                key: "col-{d}",
                                class: "sched-week-col",
                                ondoubleclick: move |_| open_editor.call(EditTarget::New(Prefill::blank(d))),
                                for h in 0..24u32 { div { key: "g{h}", class: "sched-hour-line" } }
                                for o in occs {
                                    {
                                        let ev_id = o.event_id;
                                        rsx! { TimedBlock { o: o.clone(), on_open: move |_| {
                                            if let Some(ev) = store.get(ev_id) { open_editor.call(EditTarget::Edit(ev)); }
                                        } } }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// An absolutely-positioned event block inside a day column.
#[component]
fn TimedBlock(o: Occurrence, on_open: EventHandler<()>) -> Element {
    let (top_pct, height_pct, label) = if o.all_day {
        (0.0f32, 4.0f32, format!("{} (all day)", o.title))
    } else {
        let start_min = o.start.hour() as f32 * 60.0 + o.start.minute() as f32;
        let end_min = (o.end.hour() as f32 * 60.0 + o.end.minute() as f32).max(start_min + 30.0);
        let top = start_min / 1440.0 * 100.0;
        let height = ((end_min - start_min) / 1440.0 * 100.0).max(2.0);
        (
            top,
            height,
            format!("{} {}", time_label(o.start.time()), o.title),
        )
    };
    rsx! {
        div {
            class: "sched-block",
            style: "top:{top_pct}%;height:{height_pct}%;background:{o.source.color_var()}",
            title: "{label}",
            onclick: move |_| on_open.call(()),
            "{label}"
        }
    }
}

// ---------------------------------------------------------------------------
// Day view
// ---------------------------------------------------------------------------

#[component]
fn DayView(
    store: TimeStore,
    today: NaiveDate,
    view: Signal<CalendarView>,
    open_editor: Callback<EditTarget>,
) -> Element {
    let date = match &*view.read() {
        CalendarView::Day { date } => *date,
        _ => today,
    };
    let occs = store.occurrences_on(date);
    rsx! {
        div { class: "sched-day-view",
            div { class: "sched-week-body single",
                div { class: "sched-rail",
                    for h in 0..24u32 { div { key: "{h}", class: "sched-rail-hour", "{h:02}:00" } }
                }
                div {
                    class: "sched-week-col wide",
                    ondoubleclick: move |_| open_editor.call(EditTarget::New(Prefill::blank(date))),
                    for h in 0..24u32 { div { key: "g{h}", class: "sched-hour-line" } }
                    for o in occs {
                        {
                            let ev_id = o.event_id;
                            rsx! { TimedBlock { o: o.clone(), on_open: move |_| {
                                if let Some(ev) = store.get(ev_id) { open_editor.call(EditTarget::Edit(ev)); }
                            } } }
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Year grid (12 mini-months)
// ---------------------------------------------------------------------------

#[component]
fn YearGrid(store: TimeStore, view: Signal<CalendarView>, today: NaiveDate) -> Element {
    let year = match &*view.read() {
        CalendarView::Year { year } => *year,
        _ => today.year(),
    };
    let by_date = grouped(
        &store,
        NaiveDate::from_ymd_opt(year, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(year, 12, 31).unwrap(),
    );
    rsx! {
        div { class: "sched-year",
            for m in 1..=12u32 {
                {
                    let count: usize = (1..=31)
                        .filter_map(|d| NaiveDate::from_ymd_opt(year, m, d))
                        .map(|d| by_date.get(&d).map(|v| v.len()).unwrap_or(0))
                        .sum();
                    rsx! {
                        div { key: "{m}", class: "sched-mini",
                            div {
                                class: "sched-mini-head",
                                onclick: move |_| view.set(CalendarView::Month { year, month: m }),
                                "{month_name(m)}"
                                if count > 0 { span { class: "sched-mini-count", "{count}" } }
                            }
                            MiniMonth { year, month: m, today, by_date: by_date.clone(), view }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn MiniMonth(
    year: i32,
    month: u32,
    today: NaiveDate,
    by_date: BTreeMap<NaiveDate, Vec<Occurrence>>,
    view: Signal<CalendarView>,
) -> Element {
    let first = first_of_month(year, month);
    let grid_start = week_start(first);
    rsx! {
        div { class: "sched-mini-grid",
            for wd in ["M","T","W","T","F","S","S"] { div { class: "sched-mini-dow", "{wd}" } }
            for i in 0..42i64 {
                {
                    let d = grid_start + Duration::days(i);
                    let in_month = d.month() == month && d.year() == year;
                    let has = by_date.get(&d).map(|v| !v.is_empty()).unwrap_or(false);
                    let cls = if !in_month { "sched-mini-cell out" }
                        else if d == today { "sched-mini-cell today" }
                        else { "sched-mini-cell" };
                    rsx! {
                        div {
                            key: "{d}",
                            class: "{cls}",
                            onclick: move |_| view.set(CalendarView::Day { date: d }),
                            "{d.day()}"
                            if has && in_month { span { class: "sched-mini-dot" } }
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Decade grid (10 years)
// ---------------------------------------------------------------------------

#[component]
fn DecadeGrid(view: Signal<CalendarView>, today: NaiveDate) -> Element {
    let start = match &*view.read() {
        CalendarView::Decade { start_year } => *start_year,
        _ => today.year(),
    };
    rsx! {
        div { class: "sched-decade",
            for y in start..start + 10 {
                button {
                    key: "{y}",
                    class: if y == today.year() { "sched-yearcell today" } else { "sched-yearcell" },
                    onclick: move |_| view.set(CalendarView::Year { year: y }),
                    "{y}"
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Season grid (3 mini-months)
// ---------------------------------------------------------------------------

#[component]
fn SeasonGrid(store: TimeStore, view: Signal<CalendarView>, today: NaiveDate) -> Element {
    let (year, season) = match &*view.read() {
        CalendarView::Season { year, season } => (*year, *season),
        _ => (today.year(), Season::of_month(today.month())),
    };
    let (from, to) = view.read().window();
    let by_date = grouped(&store, from, to);
    let first_month = season.first_month();
    rsx! {
        div { class: "sched-season",
            for k in 0..3u32 {
                {
                    let total = first_month + k;
                    let (y, m) = if total > 12 { (year + 1, total - 12) } else { (year, total) };
                    rsx! {
                        div { key: "{y}-{m}", class: "sched-mini",
                            div {
                                class: "sched-mini-head",
                                onclick: move |_| view.set(CalendarView::Month { year: y, month: m }),
                                "{month_name(m)} {y}"
                            }
                            MiniMonth { year: y, month: m, today, by_date: by_date.clone(), view }
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Agenda view (used for Range, and the overview strip)
// ---------------------------------------------------------------------------

#[component]
fn AgendaView(
    store: TimeStore,
    view: Signal<CalendarView>,
    open_editor: Callback<EditTarget>,
) -> Element {
    let (from, to) = view.read().window();
    let by_date = grouped(&store, from, to);
    rsx! {
        div { class: "sched-agenda",
            if by_date.is_empty() {
                div { class: "sched-empty", "Nothing scheduled in this range." }
            }
            for (date, occs) in by_date {
                div { key: "{date}", class: "sched-agenda-day",
                    div { class: "sched-agenda-date", "{date.format(\"%a, %b %-d\")}" }
                    div { class: "sched-agenda-items",
                        for o in occs {
                            {
                                let ev_id = o.event_id;
                                rsx! {
                                    div {
                                        key: "{o.event_id}-{o.start}",
                                        class: "sched-agenda-item",
                                        style: "border-left-color:{o.source.color_var()}",
                                        onclick: move |_| { if let Some(ev) = store.get(ev_id) { open_editor.call(EditTarget::Edit(ev)); } },
                                        span { class: "sched-agenda-time",
                                            if o.all_day { "all day" } else { "{time_label(o.start.time())}–{time_label(o.end.time())}" }
                                        }
                                        span { class: "sched-agenda-title", "{o.title}" }
                                        span { class: "sched-agenda-src", "{o.source.label()}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Compact upcoming list for the overview (next 14 days).
#[component]
fn AgendaStrip(store: TimeStore, today: NaiveDate) -> Element {
    let by_date = grouped(&store, today, today + Duration::days(13));
    rsx! {
        div { class: "sched-strip",
            div { class: "sched-strip-head", "Next 14 days" }
            if by_date.is_empty() { div { class: "sched-empty", "Clear schedule." } }
            for (date, occs) in by_date {
                for o in occs {
                    div { key: "{o.event_id}-{o.start}", class: "sched-strip-row",
                        span { class: "sched-dot", style: "background:{o.source.color_var()}" }
                        span { class: "sched-strip-date", "{date.format(\"%a %-d\")}" }
                        span { class: "sched-strip-time", if o.all_day { "—" } else { "{time_label(o.start.time())}" } }
                        span { class: "sched-strip-title", "{o.title}" }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Weekly planner: click-and-drag to lay down recurring time blocks
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
struct DragSel {
    day: usize,
    start: u32,
    end: u32,
}

fn slot_to_time(slot: u32) -> NaiveTime {
    let total_min = (PLAN_START_HOUR * 60) + slot * (60 / SLOTS_PER_HOUR);
    NaiveTime::from_hms_opt(total_min / 60, total_min % 60, 0).unwrap()
}

#[component]
fn WeeklyPlanner(store: TimeStore, today: NaiveDate, open_editor: Callback<EditTarget>) -> Element {
    let mut week_anchor = use_signal(|| week_start(today));
    let mut drag = use_signal(|| Option::<DragSel>::None);

    let start = *week_anchor.read();
    let by_date = grouped(&store, start, start + Duration::days(6));

    let finish_drag = use_callback(move |_: ()| {
        let sel_opt = drag.read().clone();
        if let Some(sel) = sel_opt {
            let lo = sel.start.min(sel.end);
            let hi = sel.start.max(sel.end);
            let st = slot_to_time(lo);
            let en = slot_to_time(hi + 1); // inclusive selection
            let date = *week_anchor.read() + Duration::days(sel.day as i64);
            open_editor.call(EditTarget::New(Prefill {
                source: EventSource::Manual,
                date,
                all_day: false,
                start: Some(st),
                end: Some(en),
                weekly_on: Some(sel.day as u8),
            }));
        }
        let mut drag = drag;
        drag.set(None);
    });

    rsx! {
        div { class: "sched-planner",
            div { class: "sched-header",
                button { class: "sched-btn", onclick: move |_| { let w = *week_anchor.read(); week_anchor.set(w - Duration::days(7)); }, "‹ Prev week" }
                h2 { class: "sched-title", "Week of {start.format(\"%b %-d, %Y\")}" }
                button { class: "sched-btn", onclick: move |_| { let w = *week_anchor.read(); week_anchor.set(w + Duration::days(7)); }, "Next week ›" }
            }
            p { class: "sched-hint", "Drag across a column to block out time. Release to name it — it'll be saved as a weekly-recurring event." }

            div {
                class: "sched-plan-grid",
                onmouseup: move |_| finish_drag.call(()),
                onmouseleave: move |_| finish_drag.call(()),

                // header row
                div { class: "sched-plan-corner" }
                for d in 0..7usize {
                    {
                        let date = start + Duration::days(d as i64);
                        rsx! {
                            div { key: "h{d}", class: if date == today { "sched-plan-dayhead today" } else { "sched-plan-dayhead" },
                                "{WEEKDAY_ABBR[d]}"
                                span { class: "sched-plan-dom", " {date.day()}" }
                            }
                        }
                    }
                }

                // body: one row per slot (time label + 7 day cells)
                for slot in 0..PLAN_SLOTS {
                    {
                        let t = slot_to_time(slot);
                        let show_label = t.minute() == 0;
                        rsx! {
                            div { key: "t{slot}", class: "sched-plan-time", if show_label { "{t.format(\"%-I %p\")}" } }
                            for d in 0..7usize {
                                {
                                    let in_sel = drag.read().map_or(false, |s| s.day == d && slot >= s.start.min(s.end) && slot <= s.start.max(s.end));
                                    rsx! {
                                        div {
                                            key: "c{d}-{slot}",
                                            class: if in_sel { "sched-plan-cell sel" } else { "sched-plan-cell" },
                                            onmousedown: move |_| { let mut drag = drag; drag.set(Some(DragSel { day: d, start: slot, end: slot })); },
                                            onmouseenter: move |_| {
                                                let mut drag = drag;
                                                let cur = drag.read().clone();
                                                if let Some(mut s) = cur {
                                                    if s.day == d { s.end = slot; drag.set(Some(s)); }
                                                }
                                            },
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // existing recurring/timed blocks for context (sibling of the grid)
            div { class: "sched-plan-existing",
                div { class: "sched-plan-existing-head", "This week" }
                for (date, occs) in by_date {
                    for o in occs {
                        if !o.all_day {
                            div { key: "{o.event_id}-{o.start}", class: "sched-strip-row",
                                span { class: "sched-dot", style: "background:{o.source.color_var()}" }
                                span { class: "sched-strip-date", "{date.format(\"%a\")}" }
                                span { class: "sched-strip-time", "{time_label(o.start.time())}" }
                                span { class: "sched-strip-title", "{o.title}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Event editor (create + edit, with recurrence)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum EndMode {
    Never,
    Count,
    Until,
}

#[component]
fn EventEditor(
    store: TimeStore,
    target: EditTarget,
    editing: Signal<Option<EditTarget>>,
) -> Element {
    // Seed form state from the target.
    let editing_id: Option<EventId> = match &target {
        EditTarget::Edit(e) => Some(e.id),
        EditTarget::New(_) => None,
    };

    let (
        init_title,
        init_notes,
        init_source,
        init_all_day,
        init_date,
        init_start,
        init_end,
        init_rec,
    ) = match &target {
        EditTarget::Edit(e) => {
            let (all_day, date, st, en) = match &e.when {
                When::AllDay { date } => (
                    true,
                    *date,
                    NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                    NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
                ),
                When::Timed { start, end } => (false, start.date(), start.time(), end.time()),
            };
            (
                e.title.clone(),
                e.notes.clone(),
                e.source,
                all_day,
                date,
                st,
                en,
                e.recurrence.clone(),
            )
        }
        EditTarget::New(p) => {
            let rec = p.weekly_on.map(|wd| Recurrence {
                freq: Freq::Weekly,
                interval: 1,
                by_weekday: vec![wd],
                count: None,
                until: None,
            });
            (
                String::new(),
                String::new(),
                p.source,
                p.all_day,
                p.date,
                p.start.unwrap_or(NaiveTime::from_hms_opt(9, 0, 0).unwrap()),
                p.end.unwrap_or(NaiveTime::from_hms_opt(10, 0, 0).unwrap()),
                rec,
            )
        }
    };

    let mut title = use_signal(|| init_title);
    let mut notes = use_signal(|| init_notes);
    let mut source = use_signal(|| init_source);
    let mut all_day = use_signal(|| init_all_day);
    let mut date_str = use_signal(|| init_date.format("%Y-%m-%d").to_string());
    let mut start_str = use_signal(|| init_start.format("%H:%M").to_string());
    let mut end_str = use_signal(|| init_end.format("%H:%M").to_string());

    let mut repeats = use_signal(|| init_rec.is_some());
    let mut freq = use_signal(|| init_rec.as_ref().map(|r| r.freq).unwrap_or(Freq::Weekly));
    let mut interval = use_signal(|| {
        init_rec
            .as_ref()
            .map(|r| r.interval)
            .unwrap_or(1)
            .to_string()
    });
    let init_weekdays = {
        let mut arr = [false; 7];
        if let Some(r) = &init_rec {
            for &w in &r.by_weekday {
                if (w as usize) < 7 {
                    arr[w as usize] = true;
                }
            }
        } else {
            arr[init_date.weekday().num_days_from_monday() as usize] = true;
        }
        arr
    };
    let mut weekdays = use_signal(|| init_weekdays);
    let mut end_mode = use_signal(|| match &init_rec {
        Some(r) if r.count.is_some() => EndMode::Count,
        Some(r) if r.until.is_some() => EndMode::Until,
        _ => EndMode::Never,
    });
    let mut count_str = use_signal(|| {
        init_rec
            .as_ref()
            .and_then(|r| r.count)
            .map(|c| c.to_string())
            .unwrap_or_else(|| "10".into())
    });
    let mut until_str = use_signal(|| {
        init_rec
            .as_ref()
            .and_then(|r| r.until)
            .map(|u| u.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| init_date.format("%Y-%m-%d").to_string())
    });

    let close = move || {
        let mut editing = editing;
        editing.set(None);
    };

    let save = move |_| {
        let d = NaiveDate::parse_from_str(&date_str.read(), "%Y-%m-%d").unwrap_or(init_date);
        let when = if *all_day.read() {
            When::AllDay { date: d }
        } else {
            let st = NaiveTime::parse_from_str(&start_str.read(), "%H:%M").unwrap_or(init_start);
            let en = NaiveTime::parse_from_str(&end_str.read(), "%H:%M").unwrap_or(init_end);
            let start = d.and_time(st);
            let mut end = d.and_time(en);
            if end <= start {
                end = start + Duration::hours(1);
            }
            When::Timed { start, end }
        };

        let recurrence = if *repeats.read() {
            let wd: Vec<u8> = weekdays
                .read()
                .iter()
                .enumerate()
                .filter_map(|(i, on)| if *on { Some(i as u8) } else { None })
                .collect();
            let mut rec = Recurrence {
                freq: *freq.read(),
                interval: interval.read().parse().unwrap_or(1).max(1),
                by_weekday: if *freq.read() == Freq::Weekly {
                    wd
                } else {
                    Vec::new()
                },
                count: None,
                until: None,
            };
            match *end_mode.read() {
                EndMode::Never => {}
                EndMode::Count => rec.count = count_str.read().parse().ok(),
                EndMode::Until => {
                    rec.until = NaiveDate::parse_from_str(&until_str.read(), "%Y-%m-%d").ok()
                }
            }
            Some(rec)
        } else {
            None
        };

        let ev = Event {
            id: editing_id.unwrap_or_default(), // nil for New; add_event assigns a real UUID
            title: {
                let t = title.read().trim().to_string();
                if t.is_empty() {
                    "Untitled".into()
                } else {
                    t
                }
            },
            notes: notes.read().clone(),
            when,
            source: *source.read(),
            recurrence,
            link: None,
        };

        if editing_id.is_some() {
            store.update_event(ev);
        } else {
            store.add_event(ev);
        }
        close();
    };

    let delete = move |_| {
        if let Some(id) = editing_id {
            store.remove_event(id);
        }
        close();
    };

    rsx! {
        div { class: "sched-modal-backdrop", onclick: move |_| close(),
            div { class: "sched-modal", onclick: move |e| e.stop_propagation(),
                h3 { if editing_id.is_some() { "Edit event" } else { "New event" } }

                label { class: "sched-field",
                    span { "Title" }
                    input { value: "{title}", oninput: move |e| title.set(e.value()), placeholder: "What is it?" }
                }

                label { class: "sched-field",
                    span { "Category" }
                    select {
                        value: "{source.read().label()}",
                        onchange: move |e| {
                            let s = match e.value().as_str() {
                                "Health" => EventSource::Health,
                                "Jax Brain" => EventSource::JaxBrain,
                                "Finance" => EventSource::FinCalc,
                                _ => EventSource::Manual,
                            };
                            source.set(s);
                        },
                        for s in EventSource::ALL {
                            option { value: "{s.label()}", selected: *source.read() == s, "{s.label()}" }
                        }
                    }
                }

                label { class: "sched-check",
                    input { r#type: "checkbox", checked: "{all_day}", onchange: move |e| all_day.set(e.checked()) }
                    span { "All day" }
                }

                div { class: "sched-row",
                    label { class: "sched-field",
                        span { "Date" }
                        input { r#type: "date", value: "{date_str}", oninput: move |e| date_str.set(e.value()) }
                    }
                    if !*all_day.read() {
                        label { class: "sched-field",
                            span { "Start" }
                            input { r#type: "time", value: "{start_str}", oninput: move |e| start_str.set(e.value()) }
                        }
                        label { class: "sched-field",
                            span { "End" }
                            input { r#type: "time", value: "{end_str}", oninput: move |e| end_str.set(e.value()) }
                        }
                    }
                }

                label { class: "sched-check",
                    input { r#type: "checkbox", checked: "{repeats}", onchange: move |e| repeats.set(e.checked()) }
                    span { "Repeats" }
                }

                if *repeats.read() {
                    div { class: "sched-recur",
                        div { class: "sched-row",
                            label { class: "sched-field",
                                span { "Every" }
                                input { r#type: "number", min: "1", value: "{interval}", oninput: move |e| interval.set(e.value()) }
                            }
                            label { class: "sched-field",
                                span { "Unit" }
                                select {
                                    onchange: move |e| {
                                        let f = match e.value().as_str() {
                                            "day" => Freq::Daily, "month" => Freq::Monthly, "year" => Freq::Yearly, _ => Freq::Weekly,
                                        };
                                        freq.set(f);
                                    },
                                    for f in Freq::ALL {
                                        option { value: "{f.label()}", selected: *freq.read() == f, "{f.label()}(s)" }
                                    }
                                }
                            }
                        }

                        if *freq.read() == Freq::Weekly {
                            div { class: "sched-weekday-row",
                                for i in 0..7usize {
                                    button {
                                        key: "{i}",
                                        r#type: "button",
                                        class: if weekdays.read()[i] { "sched-wd active" } else { "sched-wd" },
                                        onclick: move |_| { let mut w = weekdays.read().clone(); w[i] = !w[i]; weekdays.set(w); },
                                        "{WEEKDAY_ABBR[i]}"
                                    }
                                }
                            }
                        }

                        div { class: "sched-row",
                            label { class: "sched-field",
                                span { "Ends" }
                                select {
                                    onchange: move |e| {
                                        let m = match e.value().as_str() { "After" => EndMode::Count, "On date" => EndMode::Until, _ => EndMode::Never };
                                        end_mode.set(m);
                                    },
                                    option { value: "Never", selected: *end_mode.read() == EndMode::Never, "Never" }
                                    option { value: "After", selected: *end_mode.read() == EndMode::Count, "After N times" }
                                    option { value: "On date", selected: *end_mode.read() == EndMode::Until, "On date" }
                                }
                            }
                            if *end_mode.read() == EndMode::Count {
                                label { class: "sched-field",
                                    span { "Times" }
                                    input { r#type: "number", min: "1", value: "{count_str}", oninput: move |e| count_str.set(e.value()) }
                                }
                            }
                            if *end_mode.read() == EndMode::Until {
                                label { class: "sched-field",
                                    span { "Until" }
                                    input { r#type: "date", value: "{until_str}", oninput: move |e| until_str.set(e.value()) }
                                }
                            }
                        }
                    }
                }

                label { class: "sched-field",
                    span { "Notes" }
                    textarea { rows: "2", value: "{notes}", oninput: move |e| notes.set(e.value()) }
                }

                div { class: "sched-modal-actions",
                    if editing_id.is_some() {
                        button { class: "sched-btn sched-danger", onclick: delete, "Delete" }
                    }
                    div { class: "sched-spacer" }
                    button { class: "sched-btn", onclick: move |_| close(), "Cancel" }
                    button { class: "sched-btn sched-primary", onclick: save, "Save" }
                }
            }
        }
    }
}
