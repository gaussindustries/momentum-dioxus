use dioxus::prelude::*;
use chrono::{Duration, NaiveDate};
use time::OffsetDateTime;

use crate::components::date_picker::{DatePicker, DatePickerInput};

// --- CONSTANTS & LOGIC ---

const LBS_TO_KG: f64 = 0.453592;
const IN_TO_CM: f64 = 2.54;
const CALS_PER_LB_FAT: f64 = 3500.0;

#[derive(Clone, Copy, PartialEq)]
enum Gender {
    Male,
    Female,
}

#[derive(Clone, Copy, PartialEq)]
enum ActivityLevel {
    Sedentary = 0,
    LightlyActive = 1,
    ModeratelyActive = 2,
    VeryActive = 3,
}

impl ActivityLevel {
    fn multiplier(&self) -> f64 {
        match self {
            ActivityLevel::Sedentary => 1.2,
            ActivityLevel::LightlyActive => 1.375,
            ActivityLevel::ModeratelyActive => 1.55,
            ActivityLevel::VeryActive => 1.725,
        }
    }
}

#[derive(PartialEq)]
struct SimulationRow {
    week: usize,
    date_display: String,
    weight: f64,
    maintenance_cals: f64,
    goal_loss_lbs: f64,
    daily_cals: f64,
}

fn calculate_bmr(weight_lbs: f64, height_in: f64, age: f64, gender: Gender) -> f64 {
    let weight_kg = weight_lbs * LBS_TO_KG;
    let height_cm = height_in * IN_TO_CM;

    let s = (10.0 * weight_kg) + (6.25 * height_cm) - (5.0 * age);
    match gender {
        Gender::Male => s + 5.0,
        Gender::Female => s - 161.0,
    }
}

fn run_simulation(
    start_date: NaiveDate,
    start_weight: f64,
    target_weight: f64,
    height: f64,
    age: f64,
    gender: Gender,
    activity: ActivityLevel,
    percent_loss: f64,
) -> Vec<SimulationRow> {
    let mut rows = Vec::new();
    let mut current_weight = start_weight;
    let mut week = 0;

    if start_weight <= target_weight {
        return vec![];
    }

    while current_weight > target_weight && week < 150 {
        let bmr = calculate_bmr(current_weight, height, age, gender);
        let tdee = bmr * activity.multiplier();
        let loss_lbs = current_weight * percent_loss;

        let actual_loss = if current_weight - loss_lbs < target_weight {
            current_weight - target_weight
        } else {
            loss_lbs
        };
        
        let daily_deficit = (actual_loss * CALS_PER_LB_FAT) / 7.0;
        let daily_intake = tdee - daily_deficit;

        // Calculate date based on start_date + weeks
        let row_date = start_date + Duration::weeks(week as i64);
        let date_str = row_date.format("%b %d").to_string();

        rows.push(SimulationRow {
            week,
            date_display: date_str,
            weight: current_weight,
            maintenance_cals: tdee,
            goal_loss_lbs: actual_loss,
            daily_cals: daily_intake,
        });

        current_weight -= actual_loss;
        week += 1;
    }

    let final_bmr = calculate_bmr(current_weight, height, age, gender);
    let final_date = start_date + Duration::weeks(week as i64);
    
    rows.push(SimulationRow {
        week,
        date_display: final_date.format("%b %d").to_string(),
        weight: current_weight,
        maintenance_cals: final_bmr * activity.multiplier(),
        goal_loss_lbs: 0.0,
        daily_cals: final_bmr * activity.multiplier(),
    });

    rows
}

#[component]
pub fn Health(#[props(default)] overview: bool) -> Element {
    // FIX 1: Wrap the initial date in Some(...) to match Option<Date> type
    let mut start_date = use_signal(|| Some(OffsetDateTime::now_utc().date()));

    let mut weight = use_signal(|| 215.0);
    let mut target = use_signal(|| 170.0);
    let mut height = use_signal(|| 70.0);
    let mut age = use_signal(|| 30.0);
    let mut gender = use_signal(|| Gender::Male);
    let mut activity = use_signal(|| ActivityLevel::Sedentary);
    let mut percent_mode = use_signal(|| 0.01); 

    let simulation_data = use_memo(move || {
        // FIX 2: Unwrap the Option safely. Fallback to today if None.
        let time_date = start_date.read().unwrap_or_else(|| OffsetDateTime::now_utc().date());

        let chrono_start = NaiveDate::from_ymd_opt(
            time_date.year(), 
            time_date.month() as u32, 
            time_date.day() as u32
        ).unwrap_or_else(|| NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());

        run_simulation(
            chrono_start,
            *weight.read(),
            *target.read(),
            *height.read(),
            *age.read(),
            *gender.read(),
            *activity.read(),
            *percent_mode.read(),
        )
    });

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("./style.css") }

        div { class: "p-4 text-neutral-200 w-full h-full overflow-y-auto",
            
            if overview {
                div { "Overview mode HEALTH (Not Implemented)" }
            } else {
                div { class: "max-w-5xl mx-auto space-y-6",
                    h2 { class: "text-2xl font-bold mb-4 border-b border-neutral-700 pb-2", 
                        "Dynamic Maintenance Planner" 
                    }

                    div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 bg-neutral-900/50 p-4 rounded-lg border border-neutral-800",
                        
                        div { class: "flex flex-col gap-1",
                            label { class: "text-xs text-neutral-400 font-mono uppercase", "Start Date" }
                            DatePicker {
                                // FIX 3: Types now match (Signal<Option<Date>>)
                                selected_date: start_date,
                                on_value_change: move |d| start_date.set(d),
                                DatePickerInput {}
                            }
                        }

                        div { class: "flex flex-col gap-1",
                            label { class: "text-xs text-neutral-400 font-mono uppercase", "Current Weight (lbs)" }
                            input { type: "number", class: "bg-neutral-800 p-2 rounded border border-neutral-700 focus:border-blue-500 outline-none", value: "{weight}", oninput: move |e| weight.set(e.value().parse().unwrap_or(0.0)) }
                        }
                        div { class: "flex flex-col gap-1",
                            label { class: "text-xs text-neutral-400 font-mono uppercase", "Target Weight (lbs)" }
                            input { type: "number", class: "bg-neutral-800 p-2 rounded border border-neutral-700 focus:border-blue-500 outline-none", value: "{target}", oninput: move |e| target.set(e.value().parse().unwrap_or(0.0)) }
                        }
                        
                        div { class: "flex flex-col gap-1",
                            label { class: "text-xs text-neutral-400 font-mono uppercase", "Strategy" }
                             div { class: "flex gap-2 h-full",
                                button {
                                    class: format!("flex-1 text-xs rounded border transition-colors {}", if *percent_mode.read() == 0.005 { "bg-blue-600 border-blue-500 text-white" } else { "border-neutral-700 text-neutral-400 hover:bg-neutral-800" }),
                                    onclick: move |_| percent_mode.set(0.005), "Slow (0.5%)"
                                }
                                button {
                                    class: format!("flex-1 text-xs rounded border transition-colors {}", if *percent_mode.read() == 0.01 { "bg-orange-600 border-orange-500 text-white" } else { "border-neutral-700 text-neutral-400 hover:bg-neutral-800" }),
                                    onclick: move |_| percent_mode.set(0.01), "Fast (1.0%)"
                                }
                            }
                        }

                        div { class: "flex flex-col gap-1",
                            label { class: "text-xs text-neutral-400 font-mono uppercase", "Height (inches)" }
                            input { type: "number", class: "bg-neutral-800 p-2 rounded border border-neutral-700 focus:border-blue-500 outline-none", value: "{height}", oninput: move |e| height.set(e.value().parse().unwrap_or(0.0)) }
                        }
                        div { class: "flex flex-col gap-1",
                            label { class: "text-xs text-neutral-400 font-mono uppercase", "Age" }
                            input { type: "number", class: "bg-neutral-800 p-2 rounded border border-neutral-700 focus:border-blue-500 outline-none", value: "{age}", oninput: move |e| age.set(e.value().parse().unwrap_or(0.0)) }
                        }
                         div { class: "flex flex-col gap-1",
                            label { class: "text-xs text-neutral-400 font-mono uppercase", "Gender" }
                            select { class: "bg-neutral-800 p-2 rounded border border-neutral-700 focus:border-blue-500 outline-none", onchange: move |e| gender.set(if e.value() == "male" { Gender::Male } else { Gender::Female }),
                                option { value: "male", "Male" }
                                option { value: "female", "Female" }
                            }
                        }
                         div { class: "flex flex-col gap-1 col-span-1 md:col-span-2",
                            label { class: "text-xs text-neutral-400 font-mono uppercase", "Activity Level" }
                            select { class: "bg-neutral-800 p-2 rounded border border-neutral-700 focus:border-blue-500 outline-none",
                                onchange: move |e| { match e.value().as_str() { "1" => activity.set(ActivityLevel::LightlyActive), "2" => activity.set(ActivityLevel::ModeratelyActive), "3" => activity.set(ActivityLevel::VeryActive), _ => activity.set(ActivityLevel::Sedentary), } },
                                option { value: "0", "Sedentary (Office job, little exercise)" }
                                option { value: "1", "Light (1-2 days/wk)" }
                                option { value: "2", "Moderate (3-5 days/wk)" }
                                option { value: "3", "Heavy (6-7 days/wk)" }
                            }
                        }
                    }

                    div { class: "overflow-hidden border border-neutral-700 rounded-lg bg-neutral-900 shadow-xl",
                        div { class: "overflow-x-auto",
                            table { class: "w-full text-left text-sm",
                                thead { class: "bg-neutral-950 text-neutral-400 uppercase font-mono text-xs tracking-wider",
                                    tr {
                                        th { class: "p-3 font-medium", "Week" }
                                        th { class: "p-3 font-medium", "Weight" }
                                        th { class: "p-3 font-medium", "Target Loss" }
                                        th { class: "p-3 font-medium text-right", "Maint. (TDEE)" }
                                        th { class: "p-3 font-medium text-right", "Daily Intake" }
                                    }
                                }
                                tbody { class: "divide-y divide-neutral-800",
                                    for row in simulation_data.read().iter() {
                                        tr { class: "hover:bg-neutral-800/50 transition-colors",
                                            
                                            td { class: "p-3",
                                                div { class: "text-neutral-300 font-mono font-bold", "{row.week}" }
                                                div { class: "text-neutral-600 text-[10px] uppercase font-mono mt-0.5", "{row.date_display}" }
                                            }
                                            
                                            td { class: "p-3 font-bold text-neutral-200", 
                                                "{row.weight:.1}" 
                                                span { class: "text-neutral-500 text-xs font-normal ml-1", "lbs" }
                                            }
                                            td { class: "p-3 text-neutral-400", 
                                                if row.goal_loss_lbs <= 0.05 { 
                                                    span { class: "text-green-500 font-semibold", "Maintenance" } 
                                                } else { 
                                                    span { {format!("-{:.2}", row.goal_loss_lbs)} }
                                                }
                                            }
                                            td { class: "p-3 text-right text-neutral-400 font-mono", "{row.maintenance_cals as i32}" }
                                            td { class: "p-3 text-right font-mono",
                                                if row.daily_cals < 1500.0 && *gender.read() == Gender::Male { 
                                                    span { class: "text-red-400 font-bold", "{row.daily_cals as i32} kcal" } 
                                                } else if row.daily_cals < 1200.0 { 
                                                    span { class: "text-red-400 font-bold", "{row.daily_cals as i32} kcal" } 
                                                } else { 
                                                    span { class: "text-blue-400 font-bold", "{row.daily_cals as i32} kcal" } 
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    p { class: "text-xs text-neutral-500 max-w-2xl",
                        "Note: Calculations use the Mifflin-St Jeor equation updated weekly. Change Start Date to re-project."
                    }
                }
            }
        }
    }
}