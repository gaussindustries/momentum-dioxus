use dioxus::prelude::*;
use chrono::{Duration, NaiveDate};
use time::OffsetDateTime;
use std::collections::HashMap;

use crate::components::date_picker::{DatePicker, DatePickerInput};
use crate::components::accordion::{Accordion, AccordionContent, AccordionItem, AccordionTrigger};
use crate::components::checkbox::Checkbox;
use dioxus_primitives::checkbox::CheckboxState;

// Import our new models and utils
use crate::models::health::health::{HealthFile, DEFAULT_HEALTH_PATH, FoodVariant, MacroMode, DietConfig};
use crate::utils::json_store::{load_json, save_json, err_to_string};

// --- CONSTANTS & LOGIC ---

const LBS_TO_KG: f64 = 0.453592;
const IN_TO_CM: f64 = 2.54;
const CALS_PER_LB_FAT: f64 = 3500.0;

#[derive(Clone, Copy, PartialEq)]
enum Gender { Male, Female }

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
    // New Macro Fields per week
    p_grams: f64,
    c_grams: f64,
    f_grams: f64,
    remaining_cals: f64,
}

#[derive(Clone)]
struct HealthCtx {
    health_file: Signal<HealthFile>,
    file_path: Signal<String>,
    status: Signal<Option<String>>,
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

// Helper to calc calories based on mode
fn calc_macro_cals(mode: MacroMode, target: f64, total_cals: f64, cals_per_gram: f64) -> (f64, f64) {
    match mode {
        MacroMode::Percentage => {
            let cals = total_cals * (target / 100.0);
            (cals / cals_per_gram, cals) // (grams, cals)
        },
        MacroMode::Grams => {
            (target, target * cals_per_gram) // (grams, cals)
        }
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
    diet_config: DietConfig,
) -> Vec<SimulationRow> {
    let mut rows = Vec::new();
    let mut current_weight = start_weight;
    let mut week = 0;

    if start_weight <= target_weight { return vec![]; }

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

        // --- CALCULATE MACROS FOR THIS WEEK ---
        let (p_g, p_c) = calc_macro_cals(diet_config.protein.mode, if diet_config.protein.mode == MacroMode::Percentage { diet_config.protein.percentage } else { diet_config.protein.target_grams }, daily_intake, 4.0);
        let (c_g, c_c) = calc_macro_cals(diet_config.carbs.mode, if diet_config.carbs.mode == MacroMode::Percentage { diet_config.carbs.percentage } else { diet_config.carbs.target_grams }, daily_intake, 4.0);
        let (f_g, f_c) = calc_macro_cals(diet_config.fats.mode, if diet_config.fats.mode == MacroMode::Percentage { diet_config.fats.percentage } else { diet_config.fats.target_grams }, daily_intake, 9.0);
        
        let remaining = daily_intake - (p_c + c_c + f_c);

        let row_date = start_date + Duration::weeks(week as i64);
        
        rows.push(SimulationRow {
            week,
            date_display: row_date.format("%b %d").to_string(),
            weight: current_weight,
            maintenance_cals: tdee,
            goal_loss_lbs: actual_loss,
            daily_cals: daily_intake,
            p_grams: p_g,
            c_grams: c_g,
            f_grams: f_g,
            remaining_cals: remaining,
        });

        current_weight -= actual_loss;
        week += 1;
    }

    // Final Row
    let final_bmr = calculate_bmr(current_weight, height, age, gender);
    let final_tdee = final_bmr * activity.multiplier();
    
    // Recalc macros for maintenance level
    let (p_g, p_c) = calc_macro_cals(diet_config.protein.mode, if diet_config.protein.mode == MacroMode::Percentage { diet_config.protein.percentage } else { diet_config.protein.target_grams }, final_tdee, 4.0);
    let (c_g, c_c) = calc_macro_cals(diet_config.carbs.mode, if diet_config.carbs.mode == MacroMode::Percentage { diet_config.carbs.percentage } else { diet_config.carbs.target_grams }, final_tdee, 4.0);
    let (f_g, f_c) = calc_macro_cals(diet_config.fats.mode, if diet_config.fats.mode == MacroMode::Percentage { diet_config.fats.percentage } else { diet_config.fats.target_grams }, final_tdee, 9.0);
    let remaining = final_tdee - (p_c + c_c + f_c);

    rows.push(SimulationRow {
        week,
        date_display: (start_date + Duration::weeks(week as i64)).format("%b %d").to_string(),
        weight: current_weight,
        maintenance_cals: final_tdee,
        goal_loss_lbs: 0.0,
        daily_cals: final_tdee,
        p_grams: p_g,
        c_grams: c_g,
        f_grams: f_g,
        remaining_cals: remaining,
    });

    rows
}

#[component]
pub fn Health(#[props(default)] overview: bool) -> Element {
    let mut health_file = use_signal(HealthFile::default);
    let mut file_path = use_signal(|| DEFAULT_HEALTH_PATH.to_string());
    let mut status = use_signal(|| None::<String>);

    use_context_provider(|| HealthCtx { health_file, file_path, status });
    let ctx = use_context::<HealthCtx>();

    let mut start_date = use_signal(|| Some(OffsetDateTime::now_utc().date()));
    let mut weight = use_signal(|| 215.0);
    let mut target = use_signal(|| 170.0);
    let mut height = use_signal(|| 70.0);
    let mut age = use_signal(|| 30.0);
    let mut gender = use_signal(|| Gender::Male);
    let mut activity = use_signal(|| ActivityLevel::Sedentary);
    let mut percent_mode = use_signal(|| 0.01); 

    let mut weekly_plans = use_signal(|| HashMap::<usize, HashMap<String, f64>>::new());
    let mut expanded_week = use_signal(|| Option::<usize>::None);

    let on_load = {
        let mut health_file = ctx.health_file.clone();
        let mut status = ctx.status.clone();
        let file_path = ctx.file_path.clone();
        move |_| {
            let path = file_path.read().clone();
            match load_json::<HealthFile>(&path) {
                Ok(f) => { health_file.set(f); status.set(Some(format!("Loaded health data from {}", path))); }
                Err(e) => status.set(Some(err_to_string(e))),
            }
        }
    };

    let on_save = {
        let health_file = ctx.health_file.clone();
        let mut status = ctx.status.clone();
        let file_path = ctx.file_path.clone();
        move |_| {
            let path = file_path.read().clone();
            let data = health_file.read().clone();
            match save_json(&path, &data) {
                Ok(()) => status.set(Some(format!("Saved health data to {}", path))),
                Err(e) => status.set(Some(err_to_string(e))),
            }
        }
    };

    let simulation_data = use_memo(move || {
        let time_date = start_date.read().unwrap_or_else(|| OffsetDateTime::now_utc().date());
        let chrono_start = NaiveDate::from_ymd_opt(time_date.year(), time_date.month() as u32, time_date.day() as u32)
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        
        // Pass the CURRENT diet config into the simulation so rows update when inputs change
        let current_diet = health_file.read().diet.clone();

        run_simulation(
            chrono_start, *weight.read(), *target.read(), *height.read(), *age.read(), 
            *gender.read(), *activity.read(), *percent_mode.read(), current_diet
        )
    });

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("./style.css") }

        div { class: "p-4 text-neutral-200 w-full h-full overflow-y-auto",
            
            if overview {
                div { "Overview mode HEALTH (Not Implemented)" }
            } else {
                div { class: "flex justify-center",
                    div { class: "flex-col w-full max-w-[1200px]", 
                        
                        div { class: "flex items-center justify-between mb-4",
                            h2 { class: "text-2xl font-bold border-b border-neutral-700 pb-2 flex-1", "Health & Fitness" }
                            div { class: "flex items-center gap-2 ml-4",
                                input {
                                    class: "border border-neutral-700 px-2 py-1 bg-transparent rounded text-sm w-48",
                                    value: "{file_path.read()}",
                                    oninput: move |evt| file_path.set(evt.value().to_string())
                                }
                                button { class: "px-3 py-1 border border-neutral-700 rounded text-sm hover:bg-neutral-800", onclick: on_load, "Load" }
                                button { class: "px-3 py-1 border border-neutral-700 rounded text-sm hover:bg-neutral-800", onclick: on_save, "Save" }
                            }
                        }
                        if let Some(msg) = status.read().as_ref() { p { class: "text-xs text-blue-400 mb-4", "{msg}" } }

                        Accordion {
                            collapsible: true, allow_multiple_open: true, class: "w-full",

                            AccordionItem { index: 0usize, AccordionTrigger { class:"flex justify-center w-full bg-neutral-900/30 p-2 rounded mb-1 hover:bg-neutral-800/50", span { class: "font-bold", "Macro Inputs & Pantry" } }
                                AccordionContent {
                                    div { class: "grid grid-cols-1 lg:grid-cols-2 gap-6 p-4",
                                        div { class: "border border-neutral-700 rounded-lg p-4 bg-neutral-900/30", h3 { class: "font-bold text-lg mb-2 text-neutral-200", "Pantry Items" }
                                            div { class: "space-y-3",
                                                for (key, group) in health_file.read().nutrition_constants.iter() {
                                                    div { class: "flex items-start gap-3 p-2 border border-neutral-800 rounded hover:bg-neutral-800/50",
                                                        Checkbox {
                                                            checked: if group.enabled { CheckboxState::Checked } else { CheckboxState::Unchecked },
                                                            on_checked_change: {
                                                                let key = key.clone();
                                                                let mut health_file = health_file.clone();
                                                                move |s| { if let Some(g) = health_file.write().nutrition_constants.get_mut(&key) { g.enabled = matches!(s, CheckboxState::Checked); } }
                                                            }
                                                        }
                                                        div {
                                                            div { class: "font-semibold text-sm", "{key}" }
                                                            div { class: "text-xs text-neutral-500", "Unit: {group.unit}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        div { class: "border border-neutral-700 rounded-lg p-4 bg-neutral-900/30 space-y-4", h3 { class: "font-bold text-lg text-neutral-200", "Macro Targets" }
                                            div { class: "space-y-3",
                                                div { class: "flex items-center justify-between bg-neutral-950/50 p-2 rounded border border-neutral-800", div { class: "flex items-center gap-2", label { class: "text-xs text-neutral-400 font-bold", "Protein" } button { class: "text-[10px] px-1.5 py-0.5 rounded border border-neutral-700 hover:bg-neutral-800", onclick: { let mut h = ctx.health_file.clone(); move |_| { let m = h.read().diet.protein.mode; h.write().diet.protein.mode = if m == MacroMode::Percentage { MacroMode::Grams } else { MacroMode::Percentage }; } }, if ctx.health_file.read().diet.protein.mode == MacroMode::Percentage { "%" } else { "g" } } }, div { class: "flex items-center gap-1", input { type: "number", class: "bg-neutral-800 w-20 p-1 rounded text-sm text-right", value: if ctx.health_file.read().diet.protein.mode == MacroMode::Percentage { ctx.health_file.read().diet.protein.percentage } else { ctx.health_file.read().diet.protein.target_grams }, oninput: { let mut h = ctx.health_file.clone(); move |e| { let v = e.value().parse().unwrap_or(0.0); if h.read().diet.protein.mode == MacroMode::Percentage { h.write().diet.protein.percentage = v; } else { h.write().diet.protein.target_grams = v; } } } } } }
                                                div { class: "flex items-center justify-between bg-neutral-950/50 p-2 rounded border border-neutral-800", div { class: "flex items-center gap-2", label { class: "text-xs text-neutral-400 font-bold", "Carbs" } button { class: "text-[10px] px-1.5 py-0.5 rounded border border-neutral-700 hover:bg-neutral-800", onclick: { let mut h = ctx.health_file.clone(); move |_| { let m = h.read().diet.carbs.mode; h.write().diet.carbs.mode = if m == MacroMode::Percentage { MacroMode::Grams } else { MacroMode::Percentage }; } }, if ctx.health_file.read().diet.carbs.mode == MacroMode::Percentage { "%" } else { "g" } } }, div { class: "flex items-center gap-1", input { type: "number", class: "bg-neutral-800 w-20 p-1 rounded text-sm text-right", value: if ctx.health_file.read().diet.carbs.mode == MacroMode::Percentage { ctx.health_file.read().diet.carbs.percentage } else { ctx.health_file.read().diet.carbs.target_grams }, oninput: { let mut h = ctx.health_file.clone(); move |e| { let v = e.value().parse().unwrap_or(0.0); if h.read().diet.carbs.mode == MacroMode::Percentage { h.write().diet.carbs.percentage = v; } else { h.write().diet.carbs.target_grams = v; } } } } } }
                                                div { class: "flex items-center justify-between bg-neutral-950/50 p-2 rounded border border-neutral-800", div { class: "flex items-center gap-2", label { class: "text-xs text-neutral-400 font-bold", "Fats" } button { class: "text-[10px] px-1.5 py-0.5 rounded border border-neutral-700 hover:bg-neutral-800", onclick: { let mut h = ctx.health_file.clone(); move |_| { let m = h.read().diet.fats.mode; h.write().diet.fats.mode = if m == MacroMode::Percentage { MacroMode::Grams } else { MacroMode::Percentage }; } }, if ctx.health_file.read().diet.fats.mode == MacroMode::Percentage { "%" } else { "g" } } }, div { class: "flex items-center gap-1", input { type: "number", class: "bg-neutral-800 w-20 p-1 rounded text-sm text-right", value: if ctx.health_file.read().diet.fats.mode == MacroMode::Percentage { ctx.health_file.read().diet.fats.percentage } else { ctx.health_file.read().diet.fats.target_grams }, oninput: { let mut h = ctx.health_file.clone(); move |e| { let v = e.value().parse().unwrap_or(0.0); if h.read().diet.fats.mode == MacroMode::Percentage { h.write().diet.fats.percentage = v; } else { h.write().diet.fats.target_grams = v; } } } } } }
                                            }
                                        }
                                    }
                                }
                            }
                            AccordionItem {
                                index: 1usize,
                                AccordionTrigger {
                                    class:"flex justify-center w-full bg-neutral-900/30 p-2 rounded mb-1 hover:bg-neutral-800/50",
                                    span { class: "font-bold", "Dynamic Maintenance & Diet Planner" }
                                }
                                AccordionContent {
                                    div { class: "space-y-6 p-4",
                                        // Inputs
                                        div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 bg-neutral-900/50 p-4 rounded-lg border border-neutral-800",
                                            div { class: "flex gap-1 justify-center",
                                                div{ 
                                                    label { class: "text-xs text-neutral-400 font-mono uppercase", "Start Date" }
                                                    DatePicker { selected_date: start_date, on_value_change: move |d| start_date.set(d), DatePickerInput {} }
                                                }
                                            }
                                            div { class: "flex flex-col gap-1", 
												label { class: "text-xs text-neutral-400 font-mono uppercase", "Current Weight" } 
												input { type: "number", class: "bg-neutral-800 p-2 rounded border border-neutral-700 outline-none", 
													value: "{weight}", oninput: move |e| weight.set(e.value().parse().unwrap_or(0.0)) 
												} 
											}
                                            div { class: "flex flex-col gap-1", 
											label { class: "text-xs text-neutral-400 font-mono uppercase", "Target Weight" } 
											input { type: "number", class: "bg-neutral-800 p-2 rounded border border-neutral-700 outline-none", 
													value: "{target}", 
													oninput: move |e| target.set(e.value().parse().unwrap_or(0.0)) 
												} 
											}
                                            div { class: "flex flex-col gap-1", 
												label { class: "text-xs text-neutral-400 font-mono uppercase", "Strategy" }
												div { class: "flex gap-2 h-full", 
													button { class: format!("flex-1 text-xs rounded border transition-colors {}", 
													if *percent_mode.read() == 0.005 { "bg-blue-600 border-blue-500 text-white" 
												} else { 
													"border-neutral-700 text-neutral-400 hover:bg-neutral-800" 
												}), 
												onclick: move |_| percent_mode.set(0.005), "Slow" 
											} 
											button { class: format!("flex-1 text-xs rounded border transition-colors {}", 
												if *percent_mode.read() == 0.01 { 
													"bg-orange-600 border-orange-500 text-white" 
													} else {
														"border-neutral-700 text-neutral-400 hover:bg-neutral-800" 
													}), 
													onclick: move |_| percent_mode.set(0.01), "Fast" } 
												} 
											}
                                            div { class: "flex flex-col gap-1", 
												label { class: "text-xs text-neutral-400 font-mono uppercase", "Height (in)" } 
												input { type: "number", class: "bg-neutral-800 p-2 rounded border border-neutral-700 outline-none", 
													value: "{height}", oninput: move |e| height.set(e.value().parse().unwrap_or(0.0)) 
												} 
											}
                                            div { class: "flex flex-col gap-1", 
												label { class: "text-xs text-neutral-400 font-mono uppercase", "Age" } 
												input { type: "number", class: "bg-neutral-800 p-2 rounded border border-neutral-700 outline-none",
													value: "{age}", oninput: move |e| age.set(e.value().parse().unwrap_or(0.0)) 
												} 
											}
                                            div { class: "flex flex-col gap-1", 
												label { class: "text-xs text-neutral-400 font-mono uppercase", "Gender" } 
													select { class: "bg-neutral-800 p-2 rounded border border-neutral-700 outline-none text-black", 
													onchange: move |e| gender.set(if e.value() == "male" { Gender::Male } else { Gender::Female }), 
													option { value: "male", "Male" } 
													option { value: "female", "Female" } 
												} 
											}
                                            div { class: "flex flex-col gap-1",
												label { class: "text-xs text-neutral-400 font-mono uppercase", "Activity" } 
												select { class: "bg-neutral-800 p-2 rounded border border-neutral-700 outline-none text-black", 
												onchange: move |e| { match e.value().as_str() {
													"1" => activity.set(ActivityLevel::LightlyActive), 
													"2" => activity.set(ActivityLevel::ModeratelyActive), 
													"3" => activity.set(ActivityLevel::VeryActive), _ => activity.set(ActivityLevel::Sedentary), } 
												}, 
													option { value: "0", "Sedentary" } 
													option { value: "1", "Light" } 
													option { value: "2", "Moderate" } 
													option { value: "3", "Heavy" } 
												} 
											}
                                        }

                                        // SIMULATION TABLE
                                        div { class: "overflow-hidden border border-neutral-700 rounded-lg bg-neutral-900 shadow-xl",
                                            div { class: "overflow-x-auto",
                                                table { class: "w-full text-left text-sm border-collapse",
                                                    thead { class: "bg-neutral-950 text-neutral-400 uppercase font-mono text-xs tracking-wider",
                                                        tr {
                                                            th { class: "p-3", "Week" }
                                                            th { class: "p-3", "Weight" }
                                                            th { class: "p-3", "Loss" }
                                                            th { class: "p-3 text-right text-green-400", "Protein" }
                                                            th { class: "p-3 text-right text-blue-400", "Carbs" }
                                                            th { class: "p-3 text-right text-yellow-400", "Fats" }
                                                            th { class: "p-3 text-right text-neutral-500", "Rem." }
                                                            th { class: "p-3 text-right", "Daily Intake" }
                                                        }
                                                    }
                                                    tbody {
                                                        for row in simulation_data.read().iter() {
                                                            {
                                                                let current_week = row.week; 
                                                                rsx! {
                                                                    // MAIN ROW
                                                                    tr { 
                                                                        class: "hover:bg-neutral-800/50 transition-colors cursor-pointer border-b border-neutral-800",
                                                                        onclick: move |_| {
                                                                            if *expanded_week.read() == Some(current_week) {
                                                                                expanded_week.set(None);
                                                                            } else {
                                                                                expanded_week.set(Some(current_week));
                                                                            }
                                                                        },
                                                                        td { class: "p-3",
                                                                            div { class: "text-neutral-300 font-mono font-bold flex items-center gap-2", 
                                                                                if *expanded_week.read() == Some(row.week) { "▼" } else { "▶" }
                                                                                "{row.week}" 
                                                                            }
                                                                            div { class: "text-neutral-600 text-[10px] uppercase font-mono mt-0.5 pl-4", "{row.date_display}" }
                                                                        }
                                                                        td { class: "p-3 font-bold text-neutral-200", "{row.weight:.1}" }
                                                                        td { class: "p-3 text-neutral-400", if row.goal_loss_lbs <= 0.05 { "✓" } else { "{row.goal_loss_lbs:.1}" } }
                                                                        td { class: "p-3 text-right font-mono text-xs text-green-400", "{row.p_grams as i32}g" }
                                                                        td { class: "p-3 text-right font-mono text-xs text-blue-400", "{row.c_grams as i32}g" }
                                                                        td { class: "p-3 text-right font-mono text-xs text-yellow-400", "{row.f_grams as i32}g" }
                                                                        td { class: "p-3 text-right font-mono text-xs text-neutral-500", "{row.remaining_cals as i32}" }
                                                                        td { class: "p-3 text-right font-mono", "{row.daily_cals as i32}" }
                                                                    }

                                                                    // EXPANDED MEAL PLANNER ROW
                                                                    if *expanded_week.read() == Some(current_week) {
                                                                        tr { class: "bg-neutral-950/50",
                                                                            td { colspan: "8", class: "p-4 border-b border-neutral-700 shadow-inner",
                                                                                div { class: "flex flex-col lg:flex-row gap-6",
                                                                                    // 1. INPUTS
                                                                                    div { class: "flex-1 space-y-4",
																						h4 { class: "text-sm font-bold text-neutral-300 uppercase tracking-wide", "Week {current_week} Meal Plan" }
																						div { class: "grid grid-cols-1 md:grid-cols-2 gap-3",
																							for (key, group) in health_file.read().nutrition_constants.iter().filter(|(_, g)| g.enabled) {
																								for (v_name, variant) in &group.variants {
																									// Case 1: Nested Variants (e.g. Ground -> 90/10, 85/15)
																									if let FoodVariant::Nested(map) = variant {
																										{
																											// Create an iterator that produces elements
																											rsx! {
																												for (sub_name, _) in map {
																													{
																														let unique_key = format!("{} - {} - {}", key, v_name, sub_name);
																														// Using a block to scope the borrow for current_qty
																														let current_qty = weekly_plans.read().get(&current_week)
																															.and_then(|m| m.get(&unique_key)).cloned().unwrap_or(0.0);

																														rsx! {
																															div { class: "flex items-center justify-between bg-neutral-900 p-2 rounded border border-neutral-800",
																																div { 
																																	div { class: "text-xs font-semibold text-neutral-300", "{unique_key}" }
																																	div { class: "text-[10px] text-neutral-500", "Unit: {group.unit}" }
																																}
																																input { 
																																	type: "number", 
																																	class: "w-20 bg-neutral-800 border border-neutral-700 rounded px-2 py-1 text-right text-sm", 
																																	value: "{current_qty}",
																																	oninput: move |e| {
																																		let val = e.value().parse().unwrap_or(0.0);
																																		let mut plans = weekly_plans.write();
																																		plans.entry(current_week).or_default().insert(unique_key.clone(), val);
																																	}
																																}
																															}
																														}
																													}
																												}
																											}
																										}
																									} 
																									// Case 2: Direct Variants (e.g. Chuck Roast)
																									else {
																										{
																											let unique_key = format!("{} - {}", key, v_name);
																											let current_qty = weekly_plans.read().get(&current_week)
																												.and_then(|m| m.get(&unique_key)).cloned().unwrap_or(0.0);

																											rsx! {
																												div { class: "flex items-center justify-between bg-neutral-900 p-2 rounded border border-neutral-800",
																													div { 
																														div { class: "text-xs font-semibold text-neutral-300", "{unique_key}" }
																														div { class: "text-[10px] text-neutral-500", "Unit: {group.unit}" }
																													}
																													input { 
																														type: "number", 
																														class: "w-20 bg-neutral-800 border border-neutral-700 rounded px-2 py-1 text-right text-sm", 
																														value: "{current_qty}",
																														oninput: move |e| {
																															let val = e.value().parse().unwrap_or(0.0);
																															let mut plans = weekly_plans.write();
																															plans.entry(current_week).or_default().insert(unique_key.clone(), val);
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

                                                                                    // 2. REAL-TIME CALCULATION
                                                                                    div { class: "w-full lg:w-72 bg-neutral-900 border border-neutral-800 rounded p-4 h-fit",
                                                                                        h4 { class: "text-xs font-bold text-neutral-500 uppercase mb-3", "Nutrition Summary" }
                                                                                        {
                                                                                            let mut act_p = 0.0;
                                                                                            let mut act_c = 0.0;
                                                                                            let mut act_f = 0.0;
                                                                                            let mut act_cal = 0.0;

                                                                                            if let Some(plan) = weekly_plans.read().get(&current_week) {
                                                                                                for (food_key, qty) in plan {
                                                                                                    for (k, g) in &health_file.read().nutrition_constants {
                                                                                                        for (vn, v) in &g.variants {
                                                                                                            if let FoodVariant::Direct(info) = v {
                                                                                                                let check = format!("{} - {}", k, vn);
                                                                                                                if &check == food_key {
                                                                                                                    act_p += info.protein * qty;
                                                                                                                    act_cal += info.calories * qty;
                                                                                                                    act_f += info.total_fat * qty;
                                                                                                                    act_c += info.carbohydrates * qty;
                                                                                                                }
                                                                                                            } else if let FoodVariant::Nested(map) = v {
                                                                                                                for (sub, info) in map {
                                                                                                                    let check = format!("{} - {} - {}", k, vn, sub);
                                                                                                                    if &check == food_key {
                                                                                                                        act_p += info.protein * qty;
                                                                                                                        act_cal += info.calories * qty;
                                                                                                                        act_f += info.total_fat * qty;
                                                                                                                        act_c += info.carbohydrates * qty;
                                                                                                                    }
                                                                                                                }
                                                                                                            }
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                            }

                                                                                            let diff_p = act_p - row.p_grams;
                                                                                            let diff_c = act_c - row.c_grams;
                                                                                            let diff_f = act_f - row.f_grams;
                                                                                            let diff_cal = act_cal - row.daily_cals;

                                                                                            rsx! {
                                                                                                div { class: "space-y-2 text-sm",
                                                                                                    div { class: "flex justify-between", span { "Protein" } div { class: "text-right", div { class: if diff_p >= 0.0 { "text-green-400" } else { "text-red-400" }, "{act_p as i32} / {row.p_grams as i32}g" } } }
                                                                                                    div { class: "flex justify-between", span { "Carbs" } div { class: "text-right", div { class: if (diff_c).abs() < 5.0 { "text-blue-400" } else { "text-neutral-400" }, "{act_c as i32} / {row.c_grams as i32}g" } } }
                                                                                                    div { class: "flex justify-between", span { "Fats" } div { class: "text-right", div { class: if (diff_f).abs() < 5.0 { "text-yellow-400" } else { "text-neutral-400" }, "{act_f as i32} / {row.f_grams as i32}g" } } }
                                                                                                    div { class: "border-t border-neutral-700 my-2 pt-2 flex justify-between font-bold", span { "Calories" } span { class: if diff_cal > 0.0 { "text-red-500" } else { "text-green-500" }, "{act_cal as i32} / {row.daily_cals as i32}" } }
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
        }
    }
}