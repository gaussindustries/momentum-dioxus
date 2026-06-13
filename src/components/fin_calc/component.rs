// src/components/fin_calc/component.rs
use dioxus::prelude::*;
use futures_timer::Delay;
use std::{path::PathBuf, time::Duration};
use uuid::Uuid;

use crate::components::time::{use_time, Event, EventSource, Freq, Recurrence, When};
use crate::models::finCalc::finances::{
    AssetEntry, AssetKind, CashFlow, FinancesFile, Frequency, LiabilityEntry, LiabilityKind,
};
use crate::utils::json_store::{err_to_string, load_json, save_json};

// ---------------------------------------------------------------------------
// Formatting / parsing helpers
// ---------------------------------------------------------------------------

fn format_amount(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.abs();
    format!("{sign}${}.{:02}", abs / 100, abs % 100)
}

/// Bare value for an editable input (no `$`).
fn cents_to_input(cents: i64) -> String {
    let abs = cents.abs();
    let s = format!("{}.{:02}", abs / 100, abs % 100);
    if cents < 0 {
        format!("-{s}")
    } else {
        s
    }
}

/// Parse a typed dollar string ("45", "45.99", "$1,200.50") into cents.
fn parse_dollars(s: &str) -> Option<i64> {
    let cleaned: String = s
        .trim()
        .chars()
        .filter(|c| !matches!(c, '$' | ',' | ' '))
        .collect();
    if cleaned.is_empty() {
        return Some(0);
    }
    cleaned
        .parse::<f64>()
        .ok()
        .map(|f| (f * 100.0).round() as i64)
}

fn parse_iso_date(s: &str) -> Option<time::Date> {
    let f = time::macros::format_description!("[year]-[month]-[day]");
    time::Date::parse(s, &f).ok()
}

fn to_naive(d: time::Date) -> Option<chrono::NaiveDate> {
    chrono::NaiveDate::from_ymd_opt(d.year(), u8::from(d.month()) as u32, d.day() as u32)
}

fn freq_to_recurrence(f: Frequency) -> Option<Recurrence> {
    let (freq, interval) = match f {
        Frequency::OneTime => return None,
        Frequency::Daily => (Freq::Daily, 1),
        Frequency::Weekly => (Freq::Weekly, 1),
        Frequency::Monthly => (Freq::Monthly, 1),
        Frequency::Yearly => (Freq::Yearly, 1),
        Frequency::EveryNDays(n) => (Freq::Daily, n.max(1)),
        Frequency::EveryNMonths(n) => (Freq::Monthly, n.max(1)),
    };
    Some(Recurrence {
        freq,
        interval,
        by_weekday: Vec::new(),
        count: None,
        until: None,
    })
}

/// Desktop-only path outside the project folder, so saves don't trigger
/// `dx serve` rebuilds. Matches the scheduler module's convention.
fn default_finances_path() -> String {
    if let Some(proj) = directories::ProjectDirs::from("com", "gauss", "momentum-dioxus") {
        let mut p: PathBuf = proj.data_dir().to_path_buf();
        p.push("finances.json");
        return p.to_string_lossy().to_string();
    }
    "finances.json".to_string()
}

fn short_path(p: &str) -> String {
    p.rsplit(['/', '\\']).next().unwrap_or(p).to_string()
}

fn fmt_short_date(d: time::Date) -> String {
    format!("{}/{}", u8::from(d.month()), d.day())
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[component]
pub fn FinCalc(#[props(default)] overview: bool) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("./style.css") }
        if overview { Overview {} } else { FinCalcDetailed {} }
    }
}

/// Compact snapshot for the Overview page. Loads the persisted file once.
fn Overview() -> Element {
    let snapshot =
        use_hook(|| load_json::<FinancesFile>(default_finances_path()).unwrap_or_default());
    let net_worth = snapshot.net_worth();
    let monthly_net = snapshot.monthly_net();

    rsx! {
        div { class: "flex gap-4 text-secondary-color",
            div { class: "border rounded p-3 flex-1",
                div { class: "text-xs uppercase opacity-60", "Net worth" }
                div { class: "text-xl font-bold", "{format_amount(net_worth)}" }
            }
            div { class: "border rounded p-3 flex-1",
                div { class: "text-xs uppercase opacity-60", "Monthly net" }
                div {
                    class: if monthly_net >= 0 { "text-xl font-bold text-green-400" } else { "text-xl font-bold text-red-400" },
                    "{format_amount(monthly_net)}"
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Money input: own draft signal so parent re-renders don't disturb typing; the
// displayed value only reformats on commit (onchange), not per keystroke.
// ---------------------------------------------------------------------------

#[component]
fn MoneyInput(cents: i64, on_commit: EventHandler<i64>) -> Element {
    let mut draft = use_signal(|| cents_to_input(cents));
    rsx! {
        input {
            r#type: "text",
            class: "border px-2 py-1 w-28 bg-transparent text-right",
            value: "{draft}",
            oninput: move |e| draft.set(e.value()),
            onchange: move |e| {
                match parse_dollars(&e.value()) {
                    Some(c) => on_commit.call(c),
                    None => draft.set(cents_to_input(cents)),
                }
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Charts (hand-rendered SVG — no JS chart deps in a desktop webview)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum PieMode {
    All,
    Expenses,
    Income,
    Assets,
}
impl PieMode {
    const ALL: [PieMode; 4] = [
        PieMode::All,
        PieMode::Expenses,
        PieMode::Income,
        PieMode::Assets,
    ];
    fn label(&self) -> &'static str {
        match self {
            PieMode::All => "All",
            PieMode::Expenses => "Expenses",
            PieMode::Income => "Income",
            PieMode::Assets => "Assets",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum LineMode {
    NetWorth,
    TakeHome,
    Future,
}
impl LineMode {
    const ALL: [LineMode; 3] = [LineMode::NetWorth, LineMode::TakeHome, LineMode::Future];
    fn label(&self) -> &'static str {
        match self {
            LineMode::NetWorth => "Net worth",
            LineMode::TakeHome => "Take-home",
            LineMode::Future => "Future",
        }
    }
}

#[derive(Clone, PartialEq)]
struct PieSlice {
    d: String,
    color: String,
    label: String,
    note: String,
    full: bool,
}

/// Turn (label, cents) pairs into renderable pie slices. Returns the slices and
/// the positive total (0 ⇒ nothing to draw).
fn build_pie(items: Vec<(String, i64)>) -> (Vec<PieSlice>, i64) {
    const PALETTE: [&str; 8] = [
        "#60a5fa", "#f87171", "#34d399", "#fbbf24", "#a78bfa", "#fb923c", "#22d3ee", "#f472b6",
    ];
    let total: i64 = items.iter().map(|(_, v)| *v).filter(|v| *v > 0).sum();
    let mut slices = Vec::new();
    if total <= 0 {
        return (slices, 0);
    }
    let (cx, cy, r) = (100.0_f64, 100.0_f64, 90.0_f64);
    let mut a0 = -std::f64::consts::FRAC_PI_2; // start at 12 o'clock
    let mut idx = 0usize;
    for (label, value) in items {
        if value <= 0 {
            continue;
        }
        let frac = value as f64 / total as f64;
        let a1 = a0 + frac * std::f64::consts::TAU;
        let color = PALETTE[idx % PALETTE.len()].to_string();
        idx += 1;
        let full = frac >= 0.999;
        let d = if full {
            String::new()
        } else {
            let (x1, y1) = (cx + r * a0.cos(), cy + r * a0.sin());
            let (x2, y2) = (cx + r * a1.cos(), cy + r * a1.sin());
            let large = if (a1 - a0) > std::f64::consts::PI {
                1
            } else {
                0
            };
            format!(
                "M {cx:.2} {cy:.2} L {x1:.2} {y1:.2} A {r:.2} {r:.2} 0 {large} 1 {x2:.2} {y2:.2} Z"
            )
        };
        let note = format!("{} · {:.0}%", format_amount(value), frac * 100.0);
        slices.push(PieSlice {
            d,
            color,
            label,
            note,
            full,
        });
        a0 = a1;
    }
    (slices, total)
}

#[component]
fn PieChart(slices: Vec<PieSlice>) -> Element {
    rsx! {
        div { class: "flex gap-3 items-center flex-wrap",
            svg { width: "150", height: "150", view_box: "0 0 200 200",
                for (i, s) in slices.iter().enumerate() {
                    if s.full {
                        circle { key: "{i}", cx: "100", cy: "100", r: "90", fill: "{s.color}" }
                    } else {
                        path { key: "{i}", d: "{s.d}", fill: "{s.color}", stroke: "#0a0a0a", stroke_width: "1" }
                    }
                }
            }
            div { class: "flex flex-col gap-1",
                for (i, s) in slices.iter().enumerate() {
                    div { key: "{i}", class: "flex items-center gap-2 text-xs",
                        span { style: "width:10px;height:10px;border-radius:2px;display:inline-block;background:{s.color}" }
                        span { "{s.label}" }
                        span { class: "opacity-60", "{s.note}" }
                    }
                }
            }
        }
    }
}

struct Dot {
    cx: String,
    cy: String,
}

#[component]
fn LineChart(points: Vec<(String, f64)>, color: String) -> Element {
    if points.is_empty() {
        return rsx! {
            div { class: "text-xs opacity-60 p-6 text-center",
                "No history yet — edit a value or hit “Record point”, then check back over the coming days."
            }
        };
    }

    let (left, right, top, bottom) = (46.0_f64, 312.0_f64, 12.0_f64, 128.0_f64);
    let n = points.len();
    let mut mn = f64::INFINITY;
    let mut mx = f64::NEG_INFINITY;
    for (_, v) in &points {
        mn = mn.min(*v);
        mx = mx.max(*v);
    }
    if (mx - mn).abs() < 1e-9 {
        mn -= 1.0;
        mx += 1.0;
    }
    let x_at = |i: usize| {
        if n == 1 {
            (left + right) / 2.0
        } else {
            left + i as f64 * (right - left) / (n as f64 - 1.0)
        }
    };
    let y_at = |v: f64| bottom - (v - mn) / (mx - mn) * (bottom - top);

    let poly: String = points
        .iter()
        .enumerate()
        .map(|(i, (_, v))| format!("{:.1},{:.1}", x_at(i), y_at(*v)))
        .collect::<Vec<_>>()
        .join(" ");
    let dots: Vec<Dot> = points
        .iter()
        .enumerate()
        .map(|(i, (_, v))| Dot {
            cx: format!("{:.1}", x_at(i)),
            cy: format!("{:.1}", y_at(*v)),
        })
        .collect();
    let y_top = format_amount(mx as i64);
    let y_bot = format_amount(mn as i64);
    let x_first = points.first().unwrap().0.clone();
    let x_last = points.last().unwrap().0.clone();

    rsx! {
        svg { width: "100%", height: "150", view_box: "0 0 320 160",
            line { x1: "46", y1: "12", x2: "46", y2: "128", stroke: "#444", stroke_width: "1" }
            line { x1: "46", y1: "128", x2: "312", y2: "128", stroke: "#444", stroke_width: "1" }
            polyline { points: "{poly}", fill: "none", stroke: "{color}", stroke_width: "2" }
            for (i, dot) in dots.iter().enumerate() {
                circle { key: "{i}", cx: "{dot.cx}", cy: "{dot.cy}", r: "2.5", fill: "{color}" }
            }
            text { x: "4", y: "16", fill: "#888", font_size: "9", "{y_top}" }
            text { x: "4", y: "128", fill: "#888", font_size: "9", "{y_bot}" }
            text { x: "46", y: "142", fill: "#888", font_size: "9", "{x_first}" }
            text { x: "268", y: "142", fill: "#888", font_size: "9", "{x_last}" }
        }
    }
}

// ---------------------------------------------------------------------------
// Detailed manager
// ---------------------------------------------------------------------------

fn FinCalcDetailed() -> Element {
    let scheduler = use_time();

    let mut fin_path = use_signal(default_finances_path);
    // Load persisted data at mount, in the initializer (never write a signal
    // during render).
    let mut fin_state =
        use_signal(|| load_json::<FinancesFile>(default_finances_path()).unwrap_or_default());
    let mut status = use_signal(|| None::<String>);
    let mut save_tick = use_signal(|| 0_u64);

    // Debounced autosave: any edit bumps the tick; save 600ms after the last.
    use_effect(move || {
        let tick = *save_tick.read();
        if tick == 0 {
            return;
        }
        spawn(async move {
            Delay::new(Duration::from_millis(600)).await;
            if *save_tick.peek() != tick {
                return;
            }
            let path = fin_path.peek().clone();
            let mut data = fin_state.peek().clone();
            data.record_snapshot();
            let mut fin_state = fin_state;
            fin_state.set(data.clone());
            let mut status = status;
            match save_json(&path, &data) {
                Ok(()) => status.set(Some(format!("Auto-saved · {}", short_path(&path)))),
                Err(e) => status.set(Some(err_to_string(e))),
            }
        });
    });

    let mark_dirty = use_callback(move |_: ()| {
        let mut t = save_tick;
        let v = *t.peek();
        t.set(v + 1);
    });

    // Headline figures (each read's guard drops at the end of its statement).
    let net_worth = fin_state.read().net_worth();
    let total_assets = fin_state.read().total_assets();
    let total_liab = fin_state.read().total_liabilities();
    let m_income = fin_state.read().monthly_income();
    let m_expenses = fin_state.read().monthly_expenses();
    let m_net = m_income - m_expenses;

    // ---- Chart data -------------------------------------------------------
    let pie_mode = use_signal(|| PieMode::All);
    let line_mode = use_signal(|| LineMode::NetWorth);

    let (pie_slices, pie_total) = {
        let st = fin_state.read();
        let items: Vec<(String, i64)> = match *pie_mode.read() {
            PieMode::All => vec![
                ("Income /mo".to_string(), st.monthly_income()),
                ("Expenses /mo".to_string(), st.monthly_expenses()),
                ("Assets".to_string(), st.total_assets()),
                ("Liabilities".to_string(), st.total_liabilities()),
            ],
            PieMode::Expenses => st
                .expenses
                .iter()
                .map(|f| (f.name.clone(), f.per_month_cents()))
                .collect(),
            PieMode::Income => st
                .income
                .iter()
                .map(|f| (f.name.clone(), f.per_month_cents()))
                .collect(),
            PieMode::Assets => AssetKind::ALL
                .iter()
                .filter_map(|k| {
                    let sum: i64 = st
                        .assets
                        .iter()
                        .filter(|a| a.kind == *k)
                        .map(|a| a.value)
                        .sum();
                    (sum > 0).then(|| (k.label().to_string(), sum))
                })
                .collect(),
        };
        build_pie(items)
    };

    let line_color = match *line_mode.read() {
        LineMode::NetWorth => "#60a5fa",
        LineMode::TakeHome => "#34d399",
        LineMode::Future => "#a78bfa",
    }
    .to_string();

    let line_points: Vec<(String, f64)> = {
        let st = fin_state.read();
        match *line_mode.read() {
            LineMode::NetWorth => st
                .history
                .iter()
                .map(|s| (fmt_short_date(s.date), s.net_worth as f64))
                .collect(),
            LineMode::TakeHome => st
                .history
                .iter()
                .map(|s| (fmt_short_date(s.date), s.take_home() as f64))
                .collect(),
            LineMode::Future => {
                let base = st.net_worth();
                let step = st.monthly_net();
                (0..=12)
                    .map(|m| {
                        let label = match m {
                            0 => "now".to_string(),
                            12 => "+12 mo".to_string(),
                            _ => String::new(),
                        };
                        (label, (base + step * m as i64) as f64)
                    })
                    .collect()
            }
        }
    };

    let on_sync = move |_| {
        scheduler.remove_by_source(EventSource::FinCalc);
        let st = fin_state.peek();
        let mut count = 0u32;
        for (flow, tag) in st
            .income
            .iter()
            .map(|f| (f, "Income"))
            .chain(st.expenses.iter().map(|f| (f, "Bill")))
        {
            if flow.amount == 0 {
                continue;
            }
            let Some(date) = to_naive(flow.date) else {
                continue;
            };
            scheduler.add_event(Event {
                id: Uuid::nil(),
                title: format!("{tag}: {} ({})", flow.name, format_amount(flow.amount)),
                notes: String::new(),
                when: When::AllDay { date },
                source: EventSource::FinCalc,
                recurrence: freq_to_recurrence(flow.frequency),
                link: Some(flow.id.to_string()),
            });
            count += 1;
        }
        let mut status = status;
        status.set(Some(format!("Pushed {count} item(s) to the calendar")));
    };

    rsx! {
        div { class: "flex flex-col gap-4 text-secondary-color",

            // ---- Dashboard ------------------------------------------------
            div { class: "grid grid-cols-2 md:grid-cols-4 gap-3",
                div { class: "border rounded p-3",
                    div { class: "text-xs uppercase opacity-60", "Net worth" }
                    div { class: if net_worth >= 0 { "text-xl font-bold text-green-400" } else { "text-xl font-bold text-red-400" }, "{format_amount(net_worth)}" }
                }
                div { class: "border rounded p-3",
                    div { class: "text-xs uppercase opacity-60", "Monthly net" }
                    div { class: if m_net >= 0 { "text-xl font-bold text-green-400" } else { "text-xl font-bold text-red-400" }, "{format_amount(m_net)}" }
                }
                div { class: "border rounded p-3",
                    div { class: "text-xs uppercase opacity-60", "Monthly income" }
                    div { class: "text-xl font-bold text-green-400", "{format_amount(m_income)}" }
                }
                div { class: "border rounded p-3",
                    div { class: "text-xs uppercase opacity-60", "Monthly expenses" }
                    div { class: "text-xl font-bold text-red-400", "{format_amount(m_expenses)}" }
                }
            }
            div { class: "text-xs opacity-60",
                "Assets {format_amount(total_assets)} − Liabilities {format_amount(total_liab)}. Income & expenses are normalized to a monthly figure by their frequency."
            }

            // ---- Charts ---------------------------------------------------
            div { class: "grid md:grid-cols-2 gap-3",
                div { class: "border rounded p-3",
                    div { class: "flex items-center justify-between mb-2",
                        h3 { class: "text-lg font-semibold", "Breakdown" }
                        div { class: "flex gap-1",
                            for m in PieMode::ALL {
                                button {
                                    key: "{m.label()}",
                                    class: if *pie_mode.read() == m { "px-2 py-0.5 border rounded text-xs bg-neutral-700" } else { "px-2 py-0.5 border rounded text-xs" },
                                    onclick: move |_| { let mut pm = pie_mode; pm.set(m); },
                                    "{m.label()}"
                                }
                            }
                        }
                    }
                    if pie_total > 0 {
                        PieChart { slices: pie_slices }
                    } else {
                        div { class: "text-xs opacity-60 p-6 text-center", "Nothing with a positive value to chart yet." }
                    }
                }
                div { class: "border rounded p-3",
                    div { class: "flex items-center justify-between mb-2",
                        h3 { class: "text-lg font-semibold", "Trend" }
                        div { class: "flex gap-1",
                            for m in LineMode::ALL {
                                button {
                                    key: "{m.label()}",
                                    class: if *line_mode.read() == m { "px-2 py-0.5 border rounded text-xs bg-neutral-700" } else { "px-2 py-0.5 border rounded text-xs" },
                                    onclick: move |_| { let mut lm = line_mode; lm.set(m); },
                                    "{m.label()}"
                                }
                            }
                        }
                    }
                    LineChart { points: line_points, color: line_color }
                }
            }

            // ---- Assets ---------------------------------------------------
            section { class: "border rounded p-3 space-y-2",
                div { class: "flex items-center justify-between",
                    h3 { class: "text-lg font-semibold", "Assets" }
                    button {
                        class: "px-3 py-1 border rounded text-xs",
                        onclick: move |_| { fin_state.write().assets.push(AssetEntry::new()); mark_dirty.call(()); },
                        "+ Add asset"
                    }
                }
                table { class: "w-full text-sm border-collapse",
                    thead { tr {
                        th { class: "text-left py-1", "Name" }
                        th { class: "text-left py-1", "Type" }
                        th { class: "text-right py-1", "Value" }
                        th { class: "w-8" }
                    } }
                    tbody {
                        for a in fin_state.read().assets.iter().cloned() {
                            tr { key: "{a.id}",
                                td { class: "py-1 pr-2",
                                    input {
                                        class: "border px-2 py-1 w-full bg-transparent",
                                        value: "{a.name}",
                                        oninput: move |e| {
                                            let v = e.value();
                                            { let mut st = fin_state.write(); if let Some(x) = st.assets.iter_mut().find(|x| x.id == a.id) { x.name = v; } }
                                            mark_dirty.call(());
                                        }
                                    }
                                }
                                td { class: "py-1 pr-2",
                                    select {
                                        class: "border px-1 py-1 bg-transparent",
                                        value: "{a.kind.label()}",
                                        onchange: move |e| {
                                            let k = AssetKind::from_label(&e.value());
                                            { let mut st = fin_state.write(); if let Some(x) = st.assets.iter_mut().find(|x| x.id == a.id) { x.kind = k; } }
                                            mark_dirty.call(());
                                        },
                                        for k in AssetKind::ALL { option { value: "{k.label()}", "{k.label()}" } }
                                    }
                                }
                                td { class: "py-1 pr-2 text-right",
                                    MoneyInput {
                                        cents: a.value,
                                        on_commit: move |c| {
                                            { let mut st = fin_state.write(); if let Some(x) = st.assets.iter_mut().find(|x| x.id == a.id) { x.value = c; } }
                                            mark_dirty.call(());
                                        }
                                    }
                                }
                                td { class: "py-1 text-center",
                                    button {
                                        class: "px-2 text-red-400",
                                        onclick: move |_| { fin_state.write().assets.retain(|x| x.id != a.id); mark_dirty.call(()); },
                                        "✕"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ---- Liabilities ----------------------------------------------
            section { class: "border rounded p-3 space-y-2",
                div { class: "flex items-center justify-between",
                    h3 { class: "text-lg font-semibold", "Liabilities" }
                    button {
                        class: "px-3 py-1 border rounded text-xs",
                        onclick: move |_| { fin_state.write().liabilities.push(LiabilityEntry::new()); mark_dirty.call(()); },
                        "+ Add liability"
                    }
                }
                table { class: "w-full text-sm border-collapse",
                    thead { tr {
                        th { class: "text-left py-1", "Name" }
                        th { class: "text-left py-1", "Type" }
                        th { class: "text-right py-1", "Balance" }
                        th { class: "w-8" }
                    } }
                    tbody {
                        for l in fin_state.read().liabilities.iter().cloned() {
                            tr { key: "{l.id}",
                                td { class: "py-1 pr-2",
                                    input {
                                        class: "border px-2 py-1 w-full bg-transparent",
                                        value: "{l.name}",
                                        oninput: move |e| {
                                            let v = e.value();
                                            { let mut st = fin_state.write(); if let Some(x) = st.liabilities.iter_mut().find(|x| x.id == l.id) { x.name = v; } }
                                            mark_dirty.call(());
                                        }
                                    }
                                }
                                td { class: "py-1 pr-2",
                                    select {
                                        class: "border px-1 py-1 bg-transparent",
                                        value: "{l.kind.label()}",
                                        onchange: move |e| {
                                            let k = LiabilityKind::from_label(&e.value());
                                            { let mut st = fin_state.write(); if let Some(x) = st.liabilities.iter_mut().find(|x| x.id == l.id) { x.kind = k; } }
                                            mark_dirty.call(());
                                        },
                                        for k in LiabilityKind::ALL { option { value: "{k.label()}", "{k.label()}" } }
                                    }
                                }
                                td { class: "py-1 pr-2 text-right",
                                    MoneyInput {
                                        cents: l.balance,
                                        on_commit: move |c| {
                                            { let mut st = fin_state.write(); if let Some(x) = st.liabilities.iter_mut().find(|x| x.id == l.id) { x.balance = c; } }
                                            mark_dirty.call(());
                                        }
                                    }
                                }
                                td { class: "py-1 text-center",
                                    button {
                                        class: "px-2 text-red-400",
                                        onclick: move |_| { fin_state.write().liabilities.retain(|x| x.id != l.id); mark_dirty.call(()); },
                                        "✕"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ---- Income ---------------------------------------------------
            section { class: "border rounded p-3 space-y-2",
                div { class: "flex items-center justify-between",
                    h3 { class: "text-lg font-semibold", "Income" }
                    button {
                        class: "px-3 py-1 border rounded text-xs",
                        onclick: move |_| { fin_state.write().income.push(CashFlow::new("New income")); mark_dirty.call(()); },
                        "+ Add income"
                    }
                }
                table { class: "w-full text-sm border-collapse",
                    thead { tr {
                        th { class: "text-left py-1", "Name" }
                        th { class: "text-right py-1", "Amount" }
                        th { class: "text-left py-1 pl-2", "Frequency" }
                        th { class: "text-left py-1 pl-2", "Next date" }
                        th { class: "text-right py-1", "Per month" }
                        th { class: "w-8" }
                    } }
                    tbody {
                        for f in fin_state.read().income.iter().cloned() {
                            tr { key: "{f.id}",
                                td { class: "py-1 pr-2",
                                    input {
                                        class: "border px-2 py-1 w-full bg-transparent",
                                        value: "{f.name}",
                                        oninput: move |e| {
                                            let v = e.value();
                                            { let mut st = fin_state.write(); if let Some(x) = st.income.iter_mut().find(|x| x.id == f.id) { x.name = v; } }
                                            mark_dirty.call(());
                                        }
                                    }
                                }
                                td { class: "py-1 pr-2 text-right",
                                    MoneyInput {
                                        cents: f.amount,
                                        on_commit: move |amt| {
                                            { let mut st = fin_state.write(); if let Some(x) = st.income.iter_mut().find(|x| x.id == f.id) { x.amount = amt; } }
                                            mark_dirty.call(());
                                        }
                                    }
                                }
                                td { class: "py-1 pl-2",
                                    select {
                                        class: "border px-1 py-1 bg-transparent",
                                        value: "{f.frequency.label()}",
                                        onchange: move |e| {
                                            let fr = Frequency::from_label(&e.value());
                                            { let mut st = fin_state.write(); if let Some(x) = st.income.iter_mut().find(|x| x.id == f.id) { x.frequency = fr; } }
                                            mark_dirty.call(());
                                        },
                                        for fr in Frequency::UI { option { value: "{fr.label()}", "{fr.label()}" } }
                                    }
                                }
                                td { class: "py-1 pl-2",
                                    input {
                                        r#type: "date",
                                        class: "border px-1 py-1 bg-transparent",
                                        value: "{f.date}",
                                        onchange: move |e| {
                                            if let Some(d) = parse_iso_date(&e.value()) {
                                                { let mut st = fin_state.write(); if let Some(x) = st.income.iter_mut().find(|x| x.id == f.id) { x.date = d; } }
                                                mark_dirty.call(());
                                            }
                                        }
                                    }
                                }
                                td { class: "py-1 pr-2 text-right opacity-70", "{format_amount(f.per_month_cents())}" }
                                td { class: "py-1 text-center",
                                    button {
                                        class: "px-2 text-red-400",
                                        onclick: move |_| { fin_state.write().income.retain(|x| x.id != f.id); mark_dirty.call(()); },
                                        "✕"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ---- Expenses -------------------------------------------------
            section { class: "border rounded p-3 space-y-2",
                div { class: "flex items-center justify-between",
                    h3 { class: "text-lg font-semibold", "Expenses" }
                    button {
                        class: "px-3 py-1 border rounded text-xs",
                        onclick: move |_| { fin_state.write().expenses.push(CashFlow::new("New expense")); mark_dirty.call(()); },
                        "+ Add expense"
                    }
                }
                table { class: "w-full text-sm border-collapse",
                    thead { tr {
                        th { class: "text-left py-1", "Name" }
                        th { class: "text-right py-1", "Amount" }
                        th { class: "text-left py-1 pl-2", "Frequency" }
                        th { class: "text-left py-1 pl-2", "Next date" }
                        th { class: "text-right py-1", "Per month" }
                        th { class: "w-8" }
                    } }
                    tbody {
                        for f in fin_state.read().expenses.iter().cloned() {
                            tr { key: "{f.id}",
                                td { class: "py-1 pr-2",
                                    input {
                                        class: "border px-2 py-1 w-full bg-transparent",
                                        value: "{f.name}",
                                        oninput: move |e| {
                                            let v = e.value();
                                            { let mut st = fin_state.write(); if let Some(x) = st.expenses.iter_mut().find(|x| x.id == f.id) { x.name = v; } }
                                            mark_dirty.call(());
                                        }
                                    }
                                }
                                td { class: "py-1 pr-2 text-right",
                                    MoneyInput {
                                        cents: f.amount,
                                        on_commit: move |amt| {
                                            { let mut st = fin_state.write(); if let Some(x) = st.expenses.iter_mut().find(|x| x.id == f.id) { x.amount = amt; } }
                                            mark_dirty.call(());
                                        }
                                    }
                                }
                                td { class: "py-1 pl-2",
                                    select {
                                        class: "border px-1 py-1 bg-transparent",
                                        value: "{f.frequency.label()}",
                                        onchange: move |e| {
                                            let fr = Frequency::from_label(&e.value());
                                            { let mut st = fin_state.write(); if let Some(x) = st.expenses.iter_mut().find(|x| x.id == f.id) { x.frequency = fr; } }
                                            mark_dirty.call(());
                                        },
                                        for fr in Frequency::UI { option { value: "{fr.label()}", "{fr.label()}" } }
                                    }
                                }
                                td { class: "py-1 pl-2",
                                    input {
                                        r#type: "date",
                                        class: "border px-1 py-1 bg-transparent",
                                        value: "{f.date}",
                                        onchange: move |e| {
                                            if let Some(d) = parse_iso_date(&e.value()) {
                                                { let mut st = fin_state.write(); if let Some(x) = st.expenses.iter_mut().find(|x| x.id == f.id) { x.date = d; } }
                                                mark_dirty.call(());
                                            }
                                        }
                                    }
                                }
                                td { class: "py-1 pr-2 text-right opacity-70", "{format_amount(f.per_month_cents())}" }
                                td { class: "py-1 text-center",
                                    button {
                                        class: "px-2 text-red-400",
                                        onclick: move |_| { fin_state.write().expenses.retain(|x| x.id != f.id); mark_dirty.call(()); },
                                        "✕"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ---- File controls --------------------------------------------
            div { class: "flex items-center gap-2 text-xs pt-2 border-t border-neutral-700",
                input {
                    class: "border px-2 py-1 flex-1 bg-transparent",
                    value: "{fin_path.read()}",
                    oninput: move |e| fin_path.set(e.value()),
                }
                button {
                    class: "px-3 py-1 border rounded",
                    onclick: move |_| {
                        let path = fin_path.peek().clone();
                        let mut status = status;
                        match load_json::<FinancesFile>(&path) {
                            Ok(f) => { fin_state.set(f); status.set(Some(format!("Loaded {}", short_path(&path)))); }
                            Err(e) => status.set(Some(err_to_string(e))),
                        }
                    },
                    "Load"
                }
                button {
                    class: "px-3 py-1 border rounded",
                    onclick: move |_| {
                        let path = fin_path.peek().clone();
                        let data = fin_state.peek().clone();
                        let mut status = status;
                        match save_json(&path, &data) {
                            Ok(()) => status.set(Some(format!("Saved {}", short_path(&path)))),
                            Err(e) => status.set(Some(err_to_string(e))),
                        }
                    },
                    "Save now"
                }
                button { class: "px-3 py-1 border rounded", onclick: move |_| { fin_state.write().record_snapshot(); mark_dirty.call(()); }, "Record point" }
                button { class: "px-3 py-1 border rounded", onclick: on_sync, "Sync to calendar" }
            }

            if let Some(msg) = status.read().as_ref() {
                p { class: "text-xs text-blue-400", "{msg}" }
            }
        }
    }
}
