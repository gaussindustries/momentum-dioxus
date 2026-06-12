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
const DAYS_OF_WEEK: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

#[derive(Clone, Copy, PartialEq)]
enum Gender { Male, Female }

#[derive(Clone, Copy, PartialEq)]
enum ActivityLevel {
    Sedentary = 0,
}

impl ActivityLevel {
    fn multiplier(&self) -> f64 {
        1.2 // Baseline BMR multiplier for sedentary state. Workouts added on top.
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
    daily_deficit: f64, 
    bmr: f64,           
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

// --- HELPERS ---

/// Returns a tuple: (List of exercises to display, Average Burn Rate Per Hour for this specific session)
/// Instead of a generic average, this calculates the burn rate based on the specific
/// variations selected for this week's rotation.
fn get_exercises_and_burn(
    workouts_data: &HashMap<String, serde_json::Value>, 
    root_workout: &str, 
    week_index: usize
) -> (Vec<(String, String, Option<String>)>, f64) { 
    let mut exercises_list = Vec::new();
    let mut total_burn_rate_sum = 0.0;
    let mut exercise_count = 0.0;

    if let Some(root_data) = workouts_data.get(root_workout) {
        if let Some(focus_map) = root_data.get("focus").and_then(|f| f.as_object()) {
            
            // 1. Get all muscle groups (e.g., "chest", "triceps", "shoulders")
            let mut foci_keys: Vec<&String> = focus_map.keys().collect();
            foci_keys.sort(); // Ensure deterministic order

            // 2. Iterate EVERY muscle group to build a full routine
            for focus_key in foci_keys {
                if let Some(exercises_map) = focus_map.get(focus_key).and_then(|e| e.as_object()) {
                    let mut exercise_names: Vec<&String> = exercises_map.keys().collect();
                    if !exercise_names.is_empty() {
                        exercise_names.sort(); 
                        
                        // 3. Select specific variation based on week index
                        let ex_idx = week_index % exercise_names.len();
                        let selected_exercise_name = exercise_names[ex_idx];

                        if let Some(ex_data) = exercises_map.get(selected_exercise_name) {
                            
                            // Get Image
                            let mut img_url = None;
                            if let Some(examples) = ex_data.get("examples").and_then(|ex| ex.as_object()) {
                                if let Some((_, ex_details)) = examples.iter().next() {
                                    if let Some(url) = ex_details.get("fileLocation").and_then(|s| s.as_str()) {
                                        img_url = Some(url.to_string());
                                    }
                                }
                            }

                            // Get Calories
                            let burn_rate = ex_data.get("caloriesPerHour").and_then(|c| c.as_f64()).unwrap_or(0.0);
                            
                            // Add to calculation
                            if burn_rate > 0.0 {
                                total_burn_rate_sum += burn_rate;
                                exercise_count += 1.0;
                            }

                            exercises_list.push((focus_key.clone(), selected_exercise_name.clone(), img_url));
                        }
                    }
                }
            }
        }
    }

    // Average the burn rate of the constituent exercises to get a "Session Intensity Rate"
    let avg_session_burn = if exercise_count > 0.0 { total_burn_rate_sum / exercise_count } else { 0.0 };
    
    (exercises_list, avg_session_burn)
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

fn calc_macro_cals(mode: MacroMode, target: f64, total_cals: f64, cals_per_gram: f64) -> (f64, f64) {
    match mode {
        MacroMode::Percentage => {
            let cals = total_cals * (target / 100.0);
            (cals / cals_per_gram, cals)
        },
        MacroMode::Grams => {
            (target, target * cals_per_gram)
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
    percent_loss: f64,
    diet_config: DietConfig,
    avg_daily_workout_cals: f64,
) -> Vec<SimulationRow> {
    let mut rows = Vec::new();
    let mut current_weight = start_weight;
    let mut week = 0;

    if start_weight <= target_weight { return vec![]; }

    while current_weight > target_weight && week < 150 {
        let bmr = calculate_bmr(current_weight, height, age, gender);
        let tdee = (bmr * 1.2) + avg_daily_workout_cals;
        
        let loss_lbs = current_weight * percent_loss;
        let actual_loss = if current_weight - loss_lbs < target_weight {
            current_weight - target_weight
        } else {
            loss_lbs
        };
        
        let daily_deficit = (actual_loss * CALS_PER_LB_FAT) / 7.0;
        let daily_intake = tdee - daily_deficit;

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
            daily_deficit, // Store deficit for daily calc usage
            bmr,           // Store BMR for daily calc usage
            p_grams: p_g, c_grams: c_g, f_grams: f_g, remaining_cals: remaining,
        });

        current_weight -= actual_loss;
        week += 1;
    }

    let final_bmr = calculate_bmr(current_weight, height, age, gender);
    let final_tdee = (final_bmr * 1.2) + avg_daily_workout_cals;
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
        daily_deficit: 0.0,
        bmr: final_bmr,
        p_grams: p_g, c_grams: c_g, f_grams: f_g, remaining_cals: remaining,
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
    let mut percent_mode = use_signal(|| 0.01); 

    let mut weekly_plans = use_signal(|| HashMap::<usize, HashMap<String, f64>>::new());
    let mut expanded_week = use_signal(|| Option::<usize>::None);
    let mut daily_durations = use_signal(|| HashMap::<String, f64>::new());

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

    // Calculate daily burns individually for precise daily targeting
    // UPDATED: Now uses get_exercises_and_burn (assuming week 0 for general projection)
    let daily_burn_map = use_memo(move || {
        let file = health_file.read();
        let durations = daily_durations.read();
        let mut map = HashMap::new();
        let mut total_weekly = 0.0;
        
        let current_week_for_projection = 0; 

        for day in DAYS_OF_WEEK.iter() {
            let mut daily_cals = 0.0;
            if let Some(day_sched) = file.schedule.get(*day) {
                if let Some(root) = day_sched.overview.first() {
                    if !root.is_empty() {
                        let hrs = durations.get(*day).cloned().unwrap_or(0.0);
                        let (_, burn_rate_per_hour) = get_exercises_and_burn(&file.workouts, root, current_week_for_projection);
                        daily_cals = burn_rate_per_hour * hrs;
                    }
                }
            }
            total_weekly += daily_cals;
            map.insert(day.to_string(), daily_cals);
        }
        (map, total_weekly)
    });

    let simulation_data = use_memo(move || {
        let time_date = start_date.read().unwrap_or_else(|| OffsetDateTime::now_utc().date());
        let chrono_start = NaiveDate::from_ymd_opt(time_date.year(), time_date.month() as u32, time_date.day() as u32)
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        
        let current_diet = health_file.read().diet.clone();
        let avg_burn = daily_burn_map.read().1 / 7.0; // Use average for the high-level projection

        run_simulation(
            chrono_start, *weight.read(), *target.read(), *height.read(), *age.read(), 
            *gender.read(), *percent_mode.read(), current_diet, avg_burn
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

                        Accordion { collapsible: true, allow_multiple_open: true, class: "w-full",

                            // --- ACCORDION ITEM 0: SCHEDULE ---
                            AccordionItem { index: 0usize, AccordionTrigger { class:"flex justify-center w-full bg-neutral-900/30 p-2 rounded mb-1 hover:bg-neutral-800/50", span { class: "font-bold", "Weekly Workout Schedule" } }
                                AccordionContent {
                                    div { class: "p-4 grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4",
                                        for day in DAYS_OF_WEEK.iter() {
                                            div { class: "bg-neutral-900 border border-neutral-800 rounded p-3 flex flex-col gap-2",
                                                div { class: "font-bold text-neutral-300 border-b border-neutral-800 pb-1 mb-1", "{day}" }
                                                div { class: "flex flex-col gap-1",
                                                    label { class: "text-[10px] uppercase text-neutral-500", "Root Workout" }
                                                    select { 
                                                        class: "bg-neutral-800 border border-neutral-700 rounded p-1 text-sm outline-none",
                                                        onchange: {
                                                            let day_key = day.to_string();
                                                            let mut health_file = health_file.clone();
                                                            move |e| {
                                                                let mut file = health_file.write();
                                                                if let Some(sched) = file.schedule.get_mut(&day_key) {
                                                                    if !sched.overview.is_empty() { sched.overview[0] = e.value(); } 
                                                                    else { sched.overview.push(e.value()); }
                                                                } else {
                                                                    file.schedule.insert(day_key.clone(), crate::models::health::health::DaySchedule { overview: vec![e.value()] });
                                                                }
                                                            }
                                                        },
                                                        option { value: "", "Rest" }
                                                        for workout_key in health_file.read().workouts.keys() {
                                                            option { 
                                                                value: "{workout_key}", 
                                                                selected: if let Some(s) = health_file.read().schedule.get(*day) { s.overview.first().map(|x| x == workout_key).unwrap_or(false) } else { false },
                                                                "{workout_key}" 
                                                            }
                                                        }
                                                    }
                                                }
                                                div { class: "flex flex-col gap-1",
                                                    label { class: "text-[10px] uppercase text-neutral-500", "Duration (Hours)" }
                                                    input { 
                                                        type: "number", step: "0.1", class: "bg-neutral-800 border border-neutral-700 rounded p-1 text-sm outline-none",
                                                        value: "{daily_durations.read().get(*day).cloned().unwrap_or(0.0)}",
                                                        oninput: {
                                                            let day_key = day.to_string();
                                                            let mut durs = daily_durations.clone();
                                                            move |e| { durs.write().insert(day_key.clone(), e.value().parse().unwrap_or(0.0)); }
                                                        }
                                                    }
                                                }
                                                div { class: "text-right text-xs text-orange-400 font-mono mt-1", "+ {daily_burn_map.read().0.get(*day).cloned().unwrap_or(0.0) as i32} kcal" }
                                                
                                                // Optional: Show Quick Summary of Focus Areas
                                                if let Some(s) = health_file.read().schedule.get(*day) {
                                                    if let Some(root) = s.overview.first() {
                                                        if !root.is_empty() {
                                                            div { class: "flex flex-wrap gap-1 mt-1",
                                                                {
                                                                    // Quick fetch just to show tags in the overview card
                                                                    let (list, _) = get_exercises_and_burn(&health_file.read().workouts, root, 0);
                                                                    rsx! {
                                                                        for (focus, _, _) in list {
                                                                            span { class: "text-[8px] bg-neutral-800 px-1 rounded text-neutral-500 uppercase", "{focus}" }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        div { class: "col-span-1 md:col-span-2 lg:col-span-3 xl:col-span-4 bg-neutral-950/50 border border-neutral-800 rounded p-3 flex justify-between items-center",
                                            span { class: "text-neutral-400 font-bold", "Projected Daily Average Burn (Added to TDEE)" }
                                            span { class: "text-xl text-green-400 font-mono font-bold", "+ {(daily_burn_map.read().1 / 7.0) as i32} kcal/day" }
                                        }
                                    }
                                }
                            }

                            // --- ACCORDION ITEM 1: SIMULATION ---
                            AccordionItem { index: 1usize, AccordionTrigger { class:"flex justify-center w-full bg-neutral-900/30 p-2 rounded mb-1 hover:bg-neutral-800/50", span { class: "font-bold", "Simulation & Diet Planner" } }
                                AccordionContent {
                                    div { class: "space-y-6 p-4",
                                        // Inputs
                                        div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 bg-neutral-900/50 p-4 rounded-lg border border-neutral-800",
                                            div { class: "flex gap-1 justify-center", div{ label { class: "text-xs text-neutral-400 font-mono uppercase", "Start Date" } DatePicker { selected_date: start_date, on_value_change: move |d| start_date.set(d), DatePickerInput {} } } }
                                            div { class: "flex flex-col gap-1", label { class: "text-xs text-neutral-400 font-mono uppercase", "Current Weight" } input { type: "number", class: "bg-neutral-800 p-2 rounded border border-neutral-700 outline-none", value: "{weight}", oninput: move |e| weight.set(e.value().parse().unwrap_or(0.0)) } }
                                            div { class: "flex flex-col gap-1", label { class: "text-xs text-neutral-400 font-mono uppercase", "Target Weight" } input { type: "number", class: "bg-neutral-800 p-2 rounded border border-neutral-700 outline-none", value: "{target}", oninput: move |e| target.set(e.value().parse().unwrap_or(0.0)) } }
                                            div { class: "flex flex-col gap-1", label { class: "text-xs text-neutral-400 font-mono uppercase", "Strategy" } div { class: "flex gap-2 h-full", button { class: format!("flex-1 text-xs rounded border transition-colors {}", if *percent_mode.read() == 0.005 { "bg-blue-600 border-blue-500 text-white" } else { "border-neutral-700 text-neutral-400 hover:bg-neutral-800" }), onclick: move |_| percent_mode.set(0.005), "Slow" } button { class: format!("flex-1 text-xs rounded border transition-colors {}", if *percent_mode.read() == 0.01 { "bg-orange-600 border-orange-500 text-white" } else { "border-neutral-700 text-neutral-400 hover:bg-neutral-800" }), onclick: move |_| percent_mode.set(0.01), "Fast" } } }
                                            div { class: "flex flex-col gap-1", label { class: "text-xs text-neutral-400 font-mono uppercase", "Height (in)" } input { type: "number", class: "bg-neutral-800 p-2 rounded border border-neutral-700 outline-none", value: "{height}", oninput: move |e| height.set(e.value().parse().unwrap_or(0.0)) } }
                                            div { class: "flex flex-col gap-1", label { class: "text-xs text-neutral-400 font-mono uppercase", "Age" } input { type: "number", class: "bg-neutral-800 p-2 rounded border border-neutral-700 outline-none", value: "{age}", oninput: move |e| age.set(e.value().parse().unwrap_or(0.0)) } }
                                            div { class: "flex flex-col gap-1", label { class: "text-xs text-neutral-400 font-mono uppercase", "Gender" } select { class: "bg-neutral-800 p-2 rounded border border-neutral-700 outline-none text-neutral-200", onchange: move |e| gender.set(if e.value() == "male" { Gender::Male } else { Gender::Female }), option { value: "male", "Male" } option { value: "female", "Female" } } }
                                            div { class: "flex flex-col gap-1", label { class: "text-xs text-neutral-400 font-mono uppercase", "Base Activity" } select { class: "bg-neutral-800 p-2 rounded border border-neutral-700 outline-none text-neutral-200", disabled: true, option { value: "0", "Sedentary (1.2x)" } } }
                                        }

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
                                                            th { class: "p-3 text-right", "Daily Intake (Avg)" }
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
                                                                        onclick: move |_| { if *expanded_week.read() == Some(current_week) { expanded_week.set(None); } else { expanded_week.set(Some(current_week)); } },
                                                                        td { class: "p-3", div { class: "text-neutral-300 font-mono font-bold flex items-center gap-2", if *expanded_week.read() == Some(row.week) { "▼" } else { "▶" } "{row.week}" } div { class: "text-neutral-600 text-[10px] uppercase font-mono mt-0.5 pl-4", "{row.date_display}" } }
                                                                        td { class: "p-3 font-bold text-neutral-200", "{row.weight:.1}" }
                                                                        td { class: "p-3 text-neutral-400", if row.goal_loss_lbs <= 0.05 { "✓" } else { "{row.goal_loss_lbs:.1}" } }
                                                                        td { class: "p-3 text-right font-mono text-xs text-green-400", "{row.p_grams as i32}g" }
                                                                        td { class: "p-3 text-right font-mono text-xs text-blue-400", "{row.c_grams as i32}g" }
                                                                        td { class: "p-3 text-right font-mono text-xs text-yellow-400", "{row.f_grams as i32}g" }
                                                                        td { class: "p-3 text-right font-mono text-xs text-neutral-500", "{row.remaining_cals as i32}" }
                                                                        td { class: "p-3 text-right font-mono", "{row.daily_cals as i32}" }
                                                                    }

                                                                    // EXPANDED MEAL PLANNER & DAILY SCHEDULE
                                                                    if *expanded_week.read() == Some(current_week) {
                                                                        tr { class: "bg-neutral-950/50",
                                                                            td { colspan: "8", class: "p-4 border-b border-neutral-700 shadow-inner",
                                                                                div { class: "flex flex-col lg:flex-row gap-6",
                                                                                    
                                                                                    // 1. MEAL PLAN (Left Column)
                                                                                    div { class: "flex-1 space-y-4",
                                                                                        div { class: "grid grid-cols-1 xl:grid-cols-2 gap-6",
                                                                                            div {
                                                                                                h4 { class: "text-sm font-bold text-neutral-300 uppercase tracking-wide mb-3", "Weekly Pantry Plan" }
                                                                                                div { class: "grid grid-cols-1 gap-3 max-h-96 overflow-y-auto pr-2",
                                                                                                    for (key, group) in health_file.read().nutrition_constants.iter().filter(|(_, g)| g.enabled) {
                                                                                                        for (v_name, variant) in &group.variants {
                                                                                                            if let FoodVariant::Nested(map) = variant {
                                                                                                                { rsx! { for (sub_name, _) in map { {
                                                                                                                    let unique_key = format!("{} - {} - {}", key, v_name, sub_name);
                                                                                                                    let current_qty = weekly_plans.read().get(&current_week).and_then(|m| m.get(&unique_key)).cloned().unwrap_or(0.0);
                                                                                                                    rsx! { div { class: "flex items-center justify-between bg-neutral-900 p-2 rounded border border-neutral-800", div { div { class: "text-xs font-semibold text-neutral-300", "{unique_key}" } div { class: "text-[10px] text-neutral-500", "Unit: {group.unit}" } } input { type: "number", class: "w-20 bg-neutral-800 border border-neutral-700 rounded px-2 py-1 text-right text-sm", value: "{current_qty}", oninput: move |e| { let val = e.value().parse().unwrap_or(0.0); weekly_plans.write().entry(current_week).or_default().insert(unique_key.clone(), val); } } } }
                                                                                                                } } } }
                                                                                                            } else {
                                                                                                                {
                                                                                                                    let unique_key = format!("{} - {}", key, v_name);
                                                                                                                    let current_qty = weekly_plans.read().get(&current_week).and_then(|m| m.get(&unique_key)).cloned().unwrap_or(0.0);
                                                                                                                    rsx! { div { class: "flex items-center justify-between bg-neutral-900 p-2 rounded border border-neutral-800", div { div { class: "text-xs font-semibold text-neutral-300", "{unique_key}" } div { class: "text-[10px] text-neutral-500", "Unit: {group.unit}" } } input { type: "number", class: "w-20 bg-neutral-800 border border-neutral-700 rounded px-2 py-1 text-right text-sm", value: "{current_qty}", oninput: move |e| { let val = e.value().parse().unwrap_or(0.0); weekly_plans.write().entry(current_week).or_default().insert(unique_key.clone(), val); } } } }
                                                                                                                }
                                                                                                            }
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                            // 2. NUTRITION SUMMARY (Center/Right Panel)
div { class: "bg-neutral-900 border border-neutral-800 rounded p-4 h-fit",
    h4 { class: "text-xs font-bold text-neutral-500 uppercase mb-3", "Nutrition Summary (Daily Avg)" }
    {
        // 1. Initialize Accumulators
        let mut act_p = 0.0; 
        let mut act_c = 0.0; 
        let mut act_f = 0.0; 
        let mut act_cal = 0.0;
        
        // Micros & Sub-nutrients
        let mut act_sat_fat = 0.0;
        let mut act_mono_fat = 0.0;
        let mut act_chol = 0.0;
        let mut act_sodium = 0.0;

        // 2. Calculate Totals from Meal Plan
        if let Some(plan) = weekly_plans.read().get(&current_week) {
            for (food_key, qty) in plan {
                for (k, g) in &health_file.read().nutrition_constants {
                    for (vn, v) in &g.variants {
                        // Check Direct or Nested
                        if let FoodVariant::Direct(info) = v { 
                            let check = format!("{} - {}", k, vn); 
                            if &check == food_key { 
                                act_p += info.protein * qty; 
                                act_cal += info.calories * qty; 
                                act_f += info.total_fat * qty; 
                                act_c += info.carbohydrates * qty;
                                // Micros
                                act_sat_fat += info.saturated_fat * qty;
                                act_mono_fat += info.monounsaturated_fat * qty;
                                act_chol += info.cholesterol * qty;
                                act_sodium += info.sodium * qty;
                            } 
                        } 
                        else if let FoodVariant::Nested(map) = v { 
                            for (sub, info) in map { 
                                let check = format!("{} - {} - {}", k, vn, sub); 
                                if &check == food_key { 
                                    act_p += info.protein * qty; 
                                    act_cal += info.calories * qty; 
                                    act_f += info.total_fat * qty; 
                                    act_c += info.carbohydrates * qty;
                                    // Micros
                                    act_sat_fat += info.saturated_fat * qty;
                                    act_mono_fat += info.monounsaturated_fat * qty;
                                    act_chol += info.cholesterol * qty;
                                    act_sodium += info.sodium * qty;
                                } 
                            } 
                        }
                    }
                }
            }
        }

        // 3. Convert Weekly Total -> Daily Average
        let avg_p = act_p; 
        let avg_c = act_c; 
        let avg_f = act_f; 
        let avg_cal = act_cal;
        
        let avg_sat = act_sat_fat;
        let avg_mono = act_mono_fat;
        let avg_chol = act_chol;
        let avg_sodium = act_sodium;

        let diff_p = avg_p - row.p_grams; 
        let diff_cal = avg_cal - row.daily_cals;
        
        rsx! {
            div { class: "space-y-4",
                // MACROS SECTION
                div {
                    h5 { class: "text-[10px] font-bold text-neutral-600 border-b border-neutral-800 pb-1 mb-2", "MACROS" }
                    div { class: "space-y-2 text-sm",
                        div { class: "flex justify-between", 
                            span { "Protein" } 
                            div { class: "text-right", 
                                div { class: if diff_p >= 0.0 { "text-green-400" } else { "text-red-400" }, "{avg_p as i32} / {row.p_grams as i32}g" } 
                            } 
                        }
                        div { class: "flex justify-between", 
                            span { "Carbs" } 
                            div { class: "text-right", 
                                div { class: if (avg_c - row.c_grams).abs() < 5.0 { "text-blue-400" } else { "text-neutral-400" }, "{avg_c as i32} / {row.c_grams as i32}g" } 
                            } 
                        }
                        div { class: "flex justify-between", 
                            span { "Fats" } 
                            div { class: "text-right", 
                                div { class: if (avg_f - row.f_grams).abs() < 5.0 { "text-yellow-400" } else { "text-neutral-400" }, "{avg_f as i32} / {row.f_grams as i32}g" } 
                            } 
                        }
                        div { class: "flex justify-between font-bold pt-1", 
                            span { "Calories" } 
                            span { class: if diff_cal > 0.0 { "text-red-500" } else { "text-green-500" }, "{avg_cal as i32} / {row.daily_cals as i32}" } 
                        }
                    }
                }

                // MICROS SECTION
                div {
                    h5 { class: "text-[10px] font-bold text-neutral-600 border-b border-neutral-800 pb-1 mb-2", "MICROS & LIPIDS" }
                    div { class: "space-y-1 text-xs text-neutral-400",
                        div { class: "flex justify-between", span { "Sodium" } span { "{avg_sodium as i32} mg" } }
                        div { class: "flex justify-between", span { "Cholesterol" } span { "{avg_chol as i32} mg" } }
                        div { class: "flex justify-between", span { "Saturated Fat" } span { "{avg_sat as i32} g" } }
                        div { class: "flex justify-between", span { "Monounsat. Fat" } span { "{avg_mono as i32} g" } }
                    }
                }
            }
        }
    }
}
                                                                                        }
                                                                                    }

                                                                                    // 2. DAILY SCHEDULE & TARGETS (Right Column)
                                                                                    div { class: "flex-1 space-y-4 border-l border-neutral-800 pl-6",
                                                                                        h4 { class: "text-sm font-bold text-neutral-300 uppercase tracking-wide", "Daily Schedule & Calorie Targets" }
                                                                                        div { class: "space-y-3",
                                                                                            for day in DAYS_OF_WEEK.iter() {
                                                                                                {
                                                                                                    let file = health_file.read();
                                                                                                    let maybe_root = file.schedule.get(*day).and_then(|s| s.overview.first()).cloned();
                                                                                                    let hrs = daily_durations.read().get(*day).cloned().unwrap_or(0.0);
                                                                                                    
                                                                                                    // [UPDATE] Get Specific Exercises for THIS week (current_week)
                                                                                                    // Also returns average burn rate for the selected variations
                                                                                                    let (exercises, avg_burn_rate) = if let Some(r) = &maybe_root {
                                                                                                        if !r.is_empty() {
                                                                                                            get_exercises_and_burn(&file.workouts, r, current_week)
                                                                                                        } else { (vec![], 0.0) }
                                                                                                    } else { (vec![], 0.0) };
                                                                                                    
                                                                                                    // Calculate specific burn for this specific week's variation
                                                                                                    let specific_burn = avg_burn_rate * hrs;

                                                                                                    // Dynamic Daily Target Calculation
                                                                                                    // Base BMR * 1.2 + Specific Workout Burn - Daily Deficit
                                                                                                    let base_expenditure = row.bmr * 1.2;
                                                                                                    let total_daily_expenditure = base_expenditure + specific_burn;
                                                                                                    let specific_daily_target = total_daily_expenditure - row.daily_deficit;

                                                                                                    if let Some(root) = maybe_root {
                                                                                                        let is_empty = exercises.is_empty();
                                                                                                        
                                                                                                        rsx! {
                                                                                                            div { class: "bg-neutral-900 border border-neutral-800 rounded p-3 relative overflow-hidden",
                                                                                                                // Header: Day + Target
                                                                                                                div { class: "flex justify-between items-center border-b border-neutral-800 pb-2 mb-2",
                                                                                                                    div { 
                                                                                                                        span { class: "font-bold text-neutral-300 mr-2", "{day}" }
                                                                                                                        if !root.is_empty() { span { class: "text-[10px] px-1.5 py-0.5 rounded bg-blue-900/30 text-blue-300 border border-blue-800/50", "{root}" } }
                                                                                                                        else { span { class: "text-[10px] px-1.5 py-0.5 rounded bg-neutral-800 text-neutral-500", "Rest" } }
                                                                                                                    }
                                                                                                                    div { class: "text-right",
                                                                                                                        div { class: "text-sm font-bold text-green-400 font-mono", "{specific_daily_target as i32} kcal" }
                                                                                                                        if !root.is_empty() {
                                                                                                                            div { class: "text-[10px] text-neutral-500", "Workout: {specific_burn as i32} kcal ({hrs}h)" }
                                                                                                                        }
                                                                                                                    }
                                                                                                                }
                                                                                                                // Exercises
                                                                                                                if !is_empty {
                                                                                                                    div { class: "space-y-3",
                                                                                                                        for (focus, ex_name, img) in exercises {
                                                                                                                            div { class: "flex gap-3",
                                                                                                                                if let Some(url) = img {
                                                                                                                                    div { class: "w-16 h-16 bg-black rounded overflow-hidden flex-shrink-0 border border-neutral-700",
                                                                                                                                        img { src: "{url}", class: "w-full h-full object-cover" }
                                                                                                                                    }
                                                                                                                                }
                                                                                                                                div {
                                                                                                                                    div { class: "text-[10px] uppercase text-neutral-500 font-bold", "{focus}" }
                                                                                                                                    div { class: "text-sm text-neutral-200", "{ex_name}" }
                                                                                                                                }
                                                                                                                            }
                                                                                                                        }
                                                                                                                    }
                                                                                                                } else if !root.is_empty() {
                                                                                                                    div { class: "text-xs text-neutral-500 italic", "No specific exercises defined." }
                                                                                                                } else {
                                                                                                                    div { class: "text-xs text-neutral-600", "Rest & Recovery." }
                                                                                                                }
                                                                                                            }
                                                                                                        } 
                                                                                                    } else { rsx! {} }
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

                            AccordionItem { index: 2usize, AccordionTrigger { class:"flex justify-center w-full bg-neutral-900/30 p-2 rounded mb-1 hover:bg-neutral-800/50", span { class: "font-bold", "Macro Inputs & Pantry" } }
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
                        }
                    }
                }
            }
        }
    }
}