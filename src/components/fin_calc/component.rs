// src/components/fin_calc/component.rs
use dioxus::prelude::*;
use futures_timer::Delay;
use std::{path::PathBuf, time::Duration};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::models::finCalc::finances::FinancesFile;
use crate::utils::json_store::{err_to_string, load_json, save_json};

fn format_amount(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.abs();
    let dollars = abs / 100;
    let rem = abs % 100;
    format!("{sign}${dollars}.{rem:02}")
}

/// Desktop-only: pick a path outside the project folder to avoid dx rebuild loops.
fn default_finances_path() -> String {
    // ~/.local/share/<app>/finances.json  (Linux)
    // (directories handles Windows/macOS equivalents too)
    if let Some(proj) = directories::ProjectDirs::from("com", "gauss", "momentum-dioxus") {
        let mut p: PathBuf = proj.data_dir().to_path_buf();
        p.push("finances.json");
        return p.to_string_lossy().to_string();
    }
    // fallback
    "finances.json".to_string()
}

#[component]
pub fn FinCalc(#[props(default)] overview: bool) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("./style.css") }
        if overview { Overview {} } else { FinCalcDetailed {} }
    }
}

fn Overview() -> Element {
    rsx! { "Overview mode FINCALC" }
}

fn FinCalcDetailed() -> Element {
    // File path + state
    let mut fin_path = use_signal(default_finances_path);
    let mut fin_state = use_signal(FinancesFile::default);
    let mut status = use_signal(|| None::<String>);

    // Debounce tick: bump this on any edit
    let mut save_tick = use_signal(|| 0_u64);

    // ---- Debounced autosave effect ----
    {
        let fin_path = fin_path.clone();
        let fin_state = fin_state.clone();
        let mut status = status.clone();
        let save_tick = save_tick.clone();

        use_effect(move || {
            let tick = *save_tick.read();
            if tick == 0 {
                return;
            }

            spawn(async move {
                Delay::new(Duration::from_millis(500)).await;

                // If any newer edit happened, bail
                if *save_tick.read() != tick {
                    return;
                }

                let path = fin_path.read().clone();
                let data = fin_state.read().clone();

                match save_json(&path, &data) {
                    Ok(()) => status.set(Some(format!("Auto-saved {path}"))),
                    Err(e) => status.set(Some(err_to_string(e))),
                }
            });
        });
    }

    // Helper: bump tick (marks dirty)
    let bump_dirty = {
        let mut save_tick = save_tick.clone();
        move || {
            let t = *save_tick.read();
            save_tick.set(t + 1);
        }
    };

    let on_load = {
        let mut fin_state = fin_state.clone();
        let mut status = status.clone();
        let fin_path = fin_path.clone();
        move |_| {
            let path = fin_path.read().clone();
            match load_json::<FinancesFile>(&path) {
                Ok(f) => {
                    fin_state.set(f);
                    status.set(Some(format!("Loaded {path}")));
                }
                Err(e) => status.set(Some(err_to_string(e))),
            }
        }
    };

    let on_save_now = {
        let fin_state = fin_state.clone();
        let mut status = status.clone();
        let fin_path = fin_path.clone();
        move |_| {
            let path = fin_path.read().clone();
            let data = fin_state.read().clone();
            match save_json(&path, &data) {
                Ok(()) => status.set(Some(format!("Saved {path}"))),
                Err(e) => status.set(Some(err_to_string(e))),
            }
        }
    };

    // Add row actions
    let on_add_asset = {
        let mut fin_state = fin_state.clone();
        let mut bump_dirty = bump_dirty.clone();
        move |_| {
            fin_state.write().assets.push(crate::models::finCalc::finances::AssetEntry {
                id: Uuid::new_v4(),
                name: "new_asset".into(),
                value: 0,
            });
            bump_dirty();
        }
    };

    let on_add_income = {
        let mut fin_state = fin_state.clone();
        let mut bump_dirty = bump_dirty.clone();
        move |_| {
            let today = OffsetDateTime::now_utc().date();
            fin_state.write().income.push(crate::models::finCalc::finances::IncomeEntry {
                id: Uuid::new_v4(),
                date: today,
                source: "new_income".into(),
                amount: 0,
            });
            bump_dirty();
        }
    };

    let on_add_expense = {
        let mut fin_state = fin_state.clone();
        let mut bump_dirty = bump_dirty.clone();
        move |_| {
            let today = OffsetDateTime::now_utc().date();
            fin_state.write().expenses.push(crate::models::finCalc::finances::ExpenseEntry {
                id: Uuid::new_v4(),
                date: today,
                category: "new_expense".into(),
                amount: 0,
            });
            bump_dirty();
        }
    };

    // Totals
    let total_assets: i64 = fin_state.read().assets.iter().map(|a| a.value).sum();
    let total_income: i64 = fin_state.read().income.iter().map(|i| i.amount).sum();
    let total_expenses: i64 = fin_state.read().expenses.iter().map(|e| e.amount).sum();
    let net_month_like = total_income - total_expenses;

    rsx! {
        div { class: "flex flex-col gap-3 text-secondary-color",

            // Path + load/save
            div { class: "flex items-center gap-2",
                input {
                    class: "border px-2 py-1 flex-1 bg-transparent",
                    value: "{fin_path.read()}",
                    oninput: {
                        let mut fin_path = fin_path.clone();
                        move |evt| fin_path.set(evt.value().to_string())
                    }
                }
                button { class: "px-3 py-1 border rounded", onclick: on_load, "Load" }
                button { class: "px-3 py-1 border rounded", onclick: on_save_now, "Save" }
            }

            if let Some(msg) = status.read().as_ref() {
                p { class: "text-sm text-blue-400", "{msg}" }
            }

            div { class: "border rounded p-4 space-y-4",
                h2 { class: "text-2xl font-bold", "Finances (auto-saving JSON)" }

                div { class: "text-sm text-neutral-300",
                    "Assets: {format_amount(total_assets)} | Income: {format_amount(total_income)} | Expenses: {format_amount(total_expenses)} | Net: {format_amount(net_month_like)}"
                }

                // -------- Assets --------
                section { class: "space-y-2",
                    div { class: "flex items-center justify-between",
                        h3 { class: "text-xl font-semibold", "Assets" }
                        button { class: "px-3 py-1 border rounded text-xs", onclick: on_add_asset, "+ Add asset" }
                    }

                    table { class: "w-full text-sm border-collapse",
                        thead {
                            tr {
                                th { class: "border-b border-neutral-700 text-left py-1", "Name" }
                                th { class: "border-b border-neutral-700 text-right py-1", "Value" }
                                th { class: "border-b border-neutral-700 text-center py-1", "Adjust" }
                            }
                        }
                        tbody {
                            for (idx, a) in fin_state.read().assets.iter().enumerate() {
                                tr { key: "{a.id}",
                                    td { class: "py-1 pr-2",
                                        input {
                                            class: "border px-2 py-1 w-full bg-transparent",
                                            value: "{a.name}",
                                            oninput: {
                                                let mut fin_state = fin_state.clone();
                                                let mut bump_dirty = bump_dirty.clone();
                                                move |evt| {
                                                    fin_state.write().assets[idx].name = evt.value().to_string();
                                                    bump_dirty();
                                                }
                                            }
                                        }
                                    }
                                    td { class: "py-1 pr-2 text-right", "{format_amount(a.value)}" }
                                    td { class: "py-1 text-center space-x-1",
                                        button {
                                            class: "px-2 border rounded text-xs",
                                            onclick: {
                                                let mut fin_state = fin_state.clone();
                                                let mut bump_dirty = bump_dirty.clone();
                                                move |_| {
                                                    fin_state.write().assets[idx].value += 10_00;
                                                    bump_dirty();
                                                }
                                            },
                                            "+$10"
                                        }
                                        button {
                                            class: "px-2 border rounded text-xs",
                                            onclick: {
                                                let mut fin_state = fin_state.clone();
                                                let mut bump_dirty = bump_dirty.clone();
                                                move |_| {
                                                    fin_state.write().assets[idx].value -= 10_00;
                                                    bump_dirty();
                                                }
                                            },
                                            "-$10"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // -------- Income --------
                section { class: "space-y-2",
                    div { class: "flex items-center justify-between",
                        h3 { class: "text-xl font-semibold", "Income" }
                        button { class: "px-3 py-1 border rounded text-xs", onclick: on_add_income, "+ Add income" }
                    }

                    table { class: "w-full text-sm border-collapse",
                        thead {
                            tr {
                                th { class: "border-b border-neutral-700 text-left py-1", "Date" }
                                th { class: "border-b border-neutral-700 text-left py-1", "Source" }
                                th { class: "border-b border-neutral-700 text-right py-1", "Amount" }
                                th { class: "border-b border-neutral-700 text-center py-1", "Adjust" }
                            }
                        }
                        tbody {
                            for (idx, inc) in fin_state.read().income.iter().enumerate() {
                                tr { key: "{inc.id}",
                                    td { class: "py-1 pr-2 text-neutral-300", "{inc.date}" }
                                    td { class: "py-1 pr-2",
                                        input {
                                            class: "border px-2 py-1 w-full bg-transparent",
                                            value: "{inc.source}",
                                            oninput: {
                                                let mut fin_state = fin_state.clone();
                                                let mut bump_dirty = bump_dirty.clone();
                                                move |evt| {
                                                    fin_state.write().income[idx].source = evt.value().to_string();
                                                    bump_dirty();
                                                }
                                            }
                                        }
                                    }
                                    td { class: "py-1 pr-2 text-right", "{format_amount(inc.amount)}" }
                                    td { class: "py-1 text-center space-x-1",
                                        button {
                                            class: "px-2 border rounded text-xs",
                                            onclick: {
                                                let mut fin_state = fin_state.clone();
                                                let mut bump_dirty = bump_dirty.clone();
                                                move |_| {
                                                    fin_state.write().income[idx].amount += 10_00;
                                                    bump_dirty();
                                                }
                                            },
                                            "+$10"
                                        }
                                        button {
                                            class: "px-2 border rounded text-xs",
                                            onclick: {
                                                let mut fin_state = fin_state.clone();
                                                let mut bump_dirty = bump_dirty.clone();
                                                move |_| {
                                                    fin_state.write().income[idx].amount -= 10_00;
                                                    bump_dirty();
                                                }
                                            },
                                            "-$10"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // -------- Expenses --------
                section { class: "space-y-2",
                    div { class: "flex items-center justify-between",
                        h3 { class: "text-xl font-semibold", "Expenses" }
                        button { class: "px-3 py-1 border rounded text-xs", onclick: on_add_expense, "+ Add expense" }
                    }

                    table { class: "w-full text-sm border-collapse",
                        thead {
                            tr {
                                th { class: "border-b border-neutral-700 text-left py-1", "Date" }
                                th { class: "border-b border-neutral-700 text-left py-1", "Category" }
                                th { class: "border-b border-neutral-700 text-right py-1", "Amount" }
                                th { class: "border-b border-neutral-700 text-center py-1", "Adjust" }
                            }
                        }
                        tbody {
                            for (idx, ex) in fin_state.read().expenses.iter().enumerate() {
                                tr { key: "{ex.id}",
                                    td { class: "py-1 pr-2 text-neutral-300", "{ex.date}" }
                                    td { class: "py-1 pr-2",
                                        input {
                                            class: "border px-2 py-1 w-full bg-transparent",
                                            value: "{ex.category}",
                                            oninput: {
                                                let mut fin_state = fin_state.clone();
                                                let mut bump_dirty = bump_dirty.clone();
                                                move |evt| {
                                                    fin_state.write().expenses[idx].category = evt.value().to_string();
                                                    bump_dirty();
                                                }
                                            }
                                        }
                                    }
                                    td { class: "py-1 pr-2 text-right", "{format_amount(ex.amount)}" }
                                    td { class: "py-1 text-center space-x-1",
                                        button {
                                            class: "px-2 border rounded text-xs",
                                            onclick: {
                                                let mut fin_state = fin_state.clone();
                                                let mut bump_dirty = bump_dirty.clone();
                                                move |_| {
                                                    fin_state.write().expenses[idx].amount += 10_00;
                                                    bump_dirty();
                                                }
                                            },
                                            "+$10"
                                        }
                                        button {
                                            class: "px-2 border rounded text-xs",
                                            onclick: {
                                                let mut fin_state = fin_state.clone();
                                                let mut bump_dirty = bump_dirty.clone();
                                                move |_| {
                                                    fin_state.write().expenses[idx].amount -= 10_00;
                                                    bump_dirty();
                                                }
                                            },
                                            "-$10"
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
}
