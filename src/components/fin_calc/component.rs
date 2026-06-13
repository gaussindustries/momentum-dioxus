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
            let data = fin_state.peek().clone();
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
                button { class: "px-3 py-1 border rounded", onclick: on_sync, "Sync to calendar" }
            }

            if let Some(msg) = status.read().as_ref() {
                p { class: "text-xs text-blue-400", "{msg}" }
            }
        }
    }
}
