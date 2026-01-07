/**
 * 
 * I want to have my embedded sys/ engineering firm, live in missouri, have land, prep building a house
 * 
 * all geared toward engineering firm which you need to start ASAP
 * 
 * finish the project, ensure drafting the project in a professional manner that shows:
 * 		how well you came from concept to practice to refinement
 * 		how well tolerances are
 * 		what concepts you've realistically touched to showcase capabilities, and thus what industries it can apply to  (all really)
 * 
 * get in contact with patent attorneys, grant money ppl, 
 * 
 * 
 * 
 * 
 * 
 * 
 * 
 * 
 * 
 * 
 * 
 * 
 * 
 * 
 */
/// src/components/goals/component.rs
use dioxus::prelude::*;
use uuid::Uuid;

use dioxus_primitives::checkbox::CheckboxState;

use crate::components::accordion::{Accordion, AccordionContent, AccordionItem, AccordionTrigger};
use crate::components::checkbox::Checkbox;

use crate::models::goals::goals::{
    DEFAULT_GOALS_PATH, GoalCategory, GoalNode, GoalsFile, Metric, Smart,
};
use crate::utils::json_store::{err_to_string, load_json, save_json};

#[derive(Clone)]
struct GoalsCtx {
    goals_state: Signal<GoalsFile>,
    status: Signal<Option<String>>,

    // editing
    editing_id: Signal<Option<Uuid>>,
    draft_title: Signal<String>,
    draft_s: Signal<String>,
    draft_m: Signal<String>,
    draft_a: Signal<String>,
    draft_r: Signal<String>,
    draft_t: Signal<String>,

    // creating (root or subgoal)
    create_open: Signal<bool>,
    create_parent: Signal<Option<Uuid>>, // None => root, Some(id) => subgoal under id
    create_title: Signal<String>,
    create_category: Signal<GoalCategory>,
    create_s: Signal<String>,
    create_m: Signal<String>,
    create_a: Signal<String>,
    create_r: Signal<String>,
    create_t: Signal<String>,
    create_metric_mode: Signal<String>, // "boolean" | "numeric"
    create_unit: Signal<String>,
    create_target: Signal<String>,
}

fn cat_label(c: GoalCategory) -> &'static str {
    match c {
        GoalCategory::Health => "health",
        GoalCategory::Wealth => "wealth",
        GoalCategory::Research => "research",
        GoalCategory::Time => "time",
        GoalCategory::Other => "other",
    }
}

fn is_done(node: &GoalNode) -> bool {
    if node.completed {
        return true;
    }
    match &node.metric {
        Metric::Boolean { done } => *done,
        Metric::Numeric { current, target, .. } => current >= target,
    }
}

fn set_done(node: &mut GoalNode, done: bool) {
    node.completed = done;
    match &mut node.metric {
        Metric::Boolean { done: d } => *d = done,
        Metric::Numeric { current, target, .. } => {
            *current = if done { *target } else { 0.0 };
        }
    }
    node.touch();
}

fn find_node<'a>(nodes: &'a [GoalNode], id: Uuid) -> Option<&'a GoalNode> {
    for n in nodes {
        if n.id == id {
            return Some(n);
        }
        if let Some(found) = find_node(&n.children, id) {
            return Some(found);
        }
    }
    None
}

fn subtree_size(node: &GoalNode) -> usize {
    let mut total = 1;
    for c in &node.children {
        total += subtree_size(c);
    }
    total
}

fn seed_dummy_goals() -> GoalsFile {
    let mut f = GoalsFile::default();

    let smart_weight = Smart {
        specific: "I want to lose weight, tone down, lose unwanted fat, become fucking ripped".to_string(),
        measurable: "I am currently 214.6 [12-29-25] so i have to lose 44.6 lbs in 4 months (11.15 lbs / month | 2.78 lbs / week)".to_string(),
        achievable: "I have lost weight before, I am more than capable stopping bad habits, it's all about momentum and self discapline".to_string(),
        relevant: "it's important for find my wife, living my life to the fullest without hinderance, and I must lead by example for the coming struggle".to_string(),
        time_bound: "I want to acheive this by april 29th".to_string(),
    };

    let metric_weight = Metric::Numeric {
        unit: "lb".to_string(),
        current: 214.6,
        target: 170.0,
        clamp_0_100: false,
    };

    let mut weight_goal = GoalNode::new("weight", GoalCategory::Health, smart_weight, metric_weight);

    weight_goal.add_child(GoalNode::new(
        "hit 10k steps/day",
        GoalCategory::Health,
        Smart {
            specific: "Walk daily to increase baseline calorie burn".into(),
            measurable: "10,000 steps per day tracked".into(),
            achievable: "I can do this even on busy days by splitting walks".into(),
            relevant: "Supports fat loss and conditioning".into(),
            time_bound: "Start today; maintain for 12 weeks".into(),
        },
        Metric::Boolean { done: false },
    ));

    weight_goal.add_child(GoalNode::new(
        "keep carbs ~10%",
        GoalCategory::Health,
        Smart {
            specific: "Keep carbs limited, mostly rice around training".into(),
            measurable: "Log macros; carbs ~10% of calories".into(),
            achievable: "Food list is simple; I can repeat meals".into(),
            relevant: "Helps cut while holding appetite stable".into(),
            time_bound: "For the next 16 weeks".into(),
        },
        Metric::Boolean { done: false },
    ));

    f.roots.push(weight_goal);

    f.roots.push(GoalNode::new(
        "new business",
        GoalCategory::Wealth,
        Smart {
            specific: "Launch/scale the engineering/embedded services business with a clear offer and pipeline".into(),
            measurable: "Publish offer + portfolio, close first 3 paid engagements".into(),
            achievable: "I already build systems; package into sellable outcomes".into(),
            relevant: "Funds land + future R&D".into(),
            time_bound: "Within 90 days: first clients".into(),
        },
        Metric::Numeric {
            unit: "clients".into(),
            current: 0.0,
            target: 3.0,
            clamp_0_100: true,
        },
    ));

    f.roots.push(GoalNode::new(
        "purchase land",
        GoalCategory::Wealth,
        Smart {
            specific: "Save and prepare to purchase land suitable for building and long-term living".into(),
            measurable: "Down payment saved + shortlist 10 viable properties".into(),
            achievable: "Automate saving; strict criteria".into(),
            relevant: "Foundation for house + workshop".into(),
            time_bound: "12–18 months".into(),
        },
        Metric::Numeric {
            unit: "usd".into(),
            current: 0.0,
            target: 25_000.0,
            clamp_0_100: true,
        },
    ));

    f
}

fn clear_create_form(ctx: &GoalsCtx) {
    ctx.create_title.clone().set("".into());
    ctx.create_s.clone().set("".into());
    ctx.create_m.clone().set("".into());
    ctx.create_a.clone().set("".into());
    ctx.create_r.clone().set("".into());
    ctx.create_t.clone().set("".into());
    ctx.create_metric_mode.clone().set("boolean".into());
    ctx.create_unit.clone().set("usd".into());
    ctx.create_target.clone().set("0".into());
}


#[component]
pub fn Goals(#[props(default)] overview: bool) -> Element {
    let mut file_path = use_signal(|| DEFAULT_GOALS_PATH.to_string());
    let goals_state = use_signal(GoalsFile::default);
    let status = use_signal(|| None::<String>);

    // editing
    let editing_id = use_signal(|| None::<Uuid>);
    let draft_title = use_signal(|| "".to_string());
    let draft_s = use_signal(|| "".to_string());
    let draft_m = use_signal(|| "".to_string());
    let draft_a = use_signal(|| "".to_string());
    let draft_r = use_signal(|| "".to_string());
    let draft_t = use_signal(|| "".to_string());

    // creating
    let create_open = use_signal(|| false);
    let create_parent = use_signal(|| None::<Uuid>);
    let create_title = use_signal(|| "".to_string());
    let create_category = use_signal(|| GoalCategory::Health);
    let create_s = use_signal(|| "".to_string());
    let create_m = use_signal(|| "".to_string());
    let create_a = use_signal(|| "".to_string());
    let create_r = use_signal(|| "".to_string());
    let create_t = use_signal(|| "".to_string());
    let create_metric_mode = use_signal(|| "boolean".to_string());
    let create_unit = use_signal(|| "usd".to_string());
    let create_target = use_signal(|| "0".to_string());

    use_context_provider(|| GoalsCtx {
        goals_state,
        status,

        editing_id,
        draft_title,
        draft_s,
        draft_m,
        draft_a,
        draft_r,
        draft_t,

        create_open,
        create_parent,
        create_title,
        create_category,
        create_s,
        create_m,
        create_a,
        create_r,
        create_t,
        create_metric_mode,
        create_unit,
        create_target,
    });

    let ctx = use_context::<GoalsCtx>();

    // LOAD
    let on_load = {
        let mut goals_state = ctx.goals_state.clone();
        let mut status = ctx.status.clone();
        let file_path = file_path.clone();
        let mut editing_id = ctx.editing_id.clone();
        let mut create_open = ctx.create_open.clone();

        move |_| {
            let path = file_path.read().clone();
            match load_json::<GoalsFile>(&path) {
                Ok(f) => {
                    goals_state.set(f);
                    editing_id.set(None);
                    create_open.set(false);
                    status.set(Some(format!("Loaded goals from {}", path)));
                }
                Err(e) => status.set(Some(err_to_string(e))),
            }
        }
    };

    // SAVE
    let on_save = {
        let goals_state = ctx.goals_state.clone();
        let mut status = ctx.status.clone();
        let file_path = file_path.clone();

        move |_| {
            let path = file_path.read().clone();
            let data = goals_state.read().clone();
            match save_json(&path, &data) {
                Ok(()) => status.set(Some(format!("Saved goals to {}", path))),
                Err(e) => status.set(Some(err_to_string(e))),
            }
        }
    };

    let on_seed_dummy = {
        let mut goals_state = ctx.goals_state.clone();
        let mut status = ctx.status.clone();
        let mut editing_id = ctx.editing_id.clone();
        let mut create_open = ctx.create_open.clone();

        move |_| {
            goals_state.set(seed_dummy_goals());
            editing_id.set(None);
            create_open.set(false);
            status.set(Some("Seeded dummy goals into memory (not saved yet). Click Save.".into()));
        }
    };

    // Add root goal button
    let on_open_create_root = {
        let mut ctx = ctx.clone();
        move |_| {
            ctx.create_parent.set(None);
            ctx.create_open.set(true);
            clear_create_form(&ctx);
            ctx.status.set(Some("Creating a ROOT goal. Fill the form and click Create.".into()));
        }
    };

    if overview {
        return rsx! { div { "Overview mode Goals" } };
    }

    // stable indices across roots
    let root_items = {
        let g = ctx.goals_state.read();
        let mut items: Vec<(Uuid, usize)> = vec![];
        let mut next = 0usize;
        for r in &g.roots {
            items.push((r.id, next));
            next += subtree_size(r);
        }
        items
    };

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("./style.css") }

        div { class: "flex flex-col gap-3 text-secondary-color",

            // Path + Load/Save
            div { class: "flex items-center gap-2",
                input {
                    class: "border px-2 py-1 flex-1 bg-transparent",
                    value: "{file_path.read()}",
                    oninput: {
                        let mut file_path = file_path.clone();
                        move |evt| file_path.set(evt.value().to_string())
                    }
                }
                button { class: "px-3 py-1 border rounded", onclick: on_load, "Load" }
                button { class: "px-3 py-1 border rounded", onclick: on_save, "Save" }
                button { class: "px-3 py-1 border rounded", onclick: on_seed_dummy, "Seed dummy goals" }
                button { class: "px-3 py-1 border rounded", onclick: on_open_create_root, "+ Add Goal" }
            }

            if let Some(msg) = ctx.status.read().as_ref() {
                p { class: "text-sm text-blue-400", "{msg}" }
            }

            if *ctx.create_open.read() {
                CreateGoalForm {}
            }

            div { class: "border rounded p-4 w-full",
                h2 { class: "text-2xl font-bold mb-3", "Goals" }

                Accordion {
                    id: "goals-accordion".to_string(),
                    allow_multiple_open: true,
                    collapsible: true,

                    for (id, idx) in root_items {
                        GoalTreeItem { key: "{id}", id, depth: 0, index: idx }
                    }
                }

                p { class: "text-xs text-neutral-500 pt-3",
                    "Roots: {ctx.goals_state.read().roots.len()} (changes persist when you click Save)"
                }
            }
        }
    }
}

#[component]
fn CreateGoalForm() -> Element {
    let ctx = use_context::<GoalsCtx>();
    let is_sub = ctx.create_parent.read().is_some();

    let can_create = {
        !ctx.create_title.read().trim().is_empty()
            && !ctx.create_s.read().trim().is_empty()
            && !ctx.create_m.read().trim().is_empty()
            && !ctx.create_a.read().trim().is_empty()
            && !ctx.create_r.read().trim().is_empty()
            && !ctx.create_t.read().trim().is_empty()
            && (ctx.create_metric_mode.read().as_str() == "boolean"
                || ctx.create_target.read().trim().parse::<f64>().is_ok())
    };

    let on_cancel = {
        let mut ctx = ctx.clone();
        move |_| {
            ctx.create_open.set(false);
            ctx.create_parent.set(None);
            ctx.status.set(Some("Create cancelled.".into()));
        }
    };

    let on_create = {
        let mut ctx = ctx.clone();
        move |_| {
            if !can_create {
                ctx.status.set(Some("Fill all SMART fields and valid target (if numeric).".into()));
                return;
            }

            let smart = Smart {
                specific: ctx.create_s.read().trim().to_string(),
                measurable: ctx.create_m.read().trim().to_string(),
                achievable: ctx.create_a.read().trim().to_string(),
                relevant: ctx.create_r.read().trim().to_string(),
                time_bound: ctx.create_t.read().trim().to_string(),
            };

            let metric = if ctx.create_metric_mode.read().as_str() == "boolean" {
                Metric::Boolean { done: false }
            } else {
                let tgt = ctx.create_target.read().trim().parse::<f64>().unwrap_or(0.0);
                Metric::Numeric {
                    unit: ctx.create_unit.read().trim().to_string(),
                    current: 0.0,
                    target: tgt,
                    clamp_0_100: true,
                }
            };

            let node = GoalNode::new(
                ctx.create_title.read().trim().to_string(),
                *ctx.create_category.read(),
                smart,
                metric,
            );

            if let Some(parent_id) = *ctx.create_parent.read() {
                // add as subgoal
                if let Some(parent) = ctx.goals_state.write().find_mut(parent_id) {
                    parent.add_child(node);
                    ctx.status.set(Some("Created subgoal (remember to Save).".into()));
                } else {
                    ctx.status.set(Some("Parent goal not found.".into()));
                }
            } else {
                // add root
                ctx.goals_state.write().roots.push(node);
                ctx.status.set(Some("Created root goal (remember to Save).".into()));
            }

            ctx.create_open.set(false);
            ctx.create_parent.set(None);
            clear_create_form(&ctx);
        }
    };

    rsx! {
        div { class: "border rounded p-4 space-y-3",
            div { class: "flex items-center justify-between",
                h3 { class: "text-xl font-bold",
                    if is_sub { "Create Subgoal (SMART)" } else { "Create Goal (SMART)" }
                }
                div { class: "flex gap-2",
                    button { class: "px-3 py-1 border rounded", onclick: on_create, "Create" }
                    button { class: "px-3 py-1 border rounded", onclick: on_cancel, "Cancel" }
                }
            }

            input {
                class: "border px-2 py-1 bg-transparent w-full",
                placeholder: "Title",
                value: "{ctx.create_title.read()}",
                oninput: { let mut s = ctx.create_title.clone(); move |e| s.set(e.value()) }
            }
            div { class: "flex items-center justify-between",
				select {
					class: "border px-2 py-1 bg-red",
					onchange: {
						let mut category = ctx.create_category.clone();
						move |e| {
							category.set(match e.value().as_str() {
								"health" => GoalCategory::Health,
								"wealth" => GoalCategory::Wealth,
								"research" => GoalCategory::Research,
								"time" => GoalCategory::Time,
								_ => GoalCategory::Other,
							})
						}
					},
					option { value: "health", "Health" }
					option { value: "wealth", "Wealth" }
					option { value: "research", "Research" }
					option { value: "time", "Time" }
					option { value: "other", "Other" }
				}

				textarea { class: "border px-2 py-1 bg-transparent", rows: "2",
					placeholder: "S: Specific — what exactly is the outcome?",
					value: "{ctx.create_s.read()}",
					oninput: { let mut s = ctx.create_s.clone(); move |e| s.set(e.value()) }
				}
				textarea { class: "border px-2 py-1 bg-transparent", rows: "2",
					placeholder: "M: Measurable — how will you measure progress?",
					value: "{ctx.create_m.read()}",
					oninput: { let mut s = ctx.create_m.clone(); move |e| s.set(e.value()) }
				}
				textarea { class: "border px-2 py-1 bg-transparent", rows: "2",
					placeholder: "A: Achievable — what makes it realistic?",
					value: "{ctx.create_a.read()}",
					oninput: { let mut s = ctx.create_a.clone(); move |e| s.set(e.value()) }
				}
				textarea { class: "border px-2 py-1 bg-transparent", rows: "2",
					placeholder: "R: Relevant — why does it matter / what does it support?",
					value: "{ctx.create_r.read()}",
					oninput: { let mut s = ctx.create_r.clone(); move |e| s.set(e.value()) }
				}
				textarea { class: "border px-2 py-1 bg-transparent", rows: "2",
					placeholder: "T: Time-bound — by when / cadence?",
					value: "{ctx.create_t.read()}",
					oninput: { let mut s = ctx.create_t.clone(); move |e| s.set(e.value()) }
				}

				div { class: "flex items-center gap-2",
					select {
						class: "border px-2 py-1 bg-transparent",
						onchange: { let mut mm = ctx.create_metric_mode.clone(); move |e| mm.set(e.value()) },
						option { value: "boolean", "Checkbox (done / not)" }
						option { value: "numeric", "Numeric (progress)" }
					}

					if ctx.create_metric_mode.read().as_str() == "numeric" {
						input {
							class: "border px-2 py-1 bg-transparent w-24",
							placeholder: "unit",
							value: "{ctx.create_unit.read()}",
							oninput: { let mut u = ctx.create_unit.clone(); move |e| u.set(e.value()) }
						}
						input {
							class: "border px-2 py-1 bg-transparent w-24",
							placeholder: "target",
							value: "{ctx.create_target.read()}",
							oninput: { let mut t = ctx.create_target.clone(); move |e| t.set(e.value()) }
						}
					}
				}
			}

            if !can_create {
                p { class: "text-xs text-neutral-400",
                    "Fill title + all SMART fields. If numeric, target must be a number."
                }
            }
        }
    }
}

#[component]
fn GoalTreeItem(id: Uuid, depth: usize, index: usize) -> Element {
    let ctx = use_context::<GoalsCtx>();

    let node_opt = {
        let g = ctx.goals_state.read();
        find_node(&g.roots, id).cloned()
    };

    let Some(node) = node_opt else {
        return rsx! { div {} };
    };

    let is_editing = *ctx.editing_id.read() == Some(id);

    // primitives expects ReadSignal<Option<CheckboxState>>
    let done_now = is_done(&node);
    let mut checked_sig: Signal<Option<CheckboxState>> = use_signal(|| {
        Some(if done_now { CheckboxState::Checked } else { CheckboxState::Unchecked })
    });

    // keep checkbox synced with model
    {
        let mut checked_sig = checked_sig.clone();
        let desired = Some(if done_now { CheckboxState::Checked } else { CheckboxState::Unchecked });
        use_effect(move || {
            if *checked_sig.read() != desired {
                checked_sig.set(desired);
            }
        });
    }

    // children indices
    let child_items = {
        let mut v: Vec<(Uuid, usize)> = vec![];
        let mut next = index + 1;
        for c in &node.children {
            v.push((c.id, next));
            next += subtree_size(c);
        }
        v
    };

    let indent_px = (depth * 14) as i32;
    let cat = cat_label(node.category);
    let time_hint = node.smart.time_bound.clone();
	let parent_title = node.title.clone();
    let parent_category = node.category;
    // Add subgoal button
     let on_open_create_sub = {
        let mut ctx = ctx.clone();
        move |_| {
            ctx.create_parent.set(Some(id));
            ctx.create_open.set(true);
            // Use the copied category
            ctx.create_category.set(parent_category);
            clear_create_form(&ctx);
            // Use the cloned title
            ctx.status.set(Some(format!("Creating a SUBGOAL under \"{}\".", parent_title)));
        }
    };

    rsx! {
        div { class: "w-full",
            div {
                class: "flex items-start w-full" ,
                style: format!("padding-left: {}px;", indent_px),

                if depth > 0 {
                    div { class: "mr-2 mt-2",
                        div { class: "w-3 h-3 border-l border-b border-neutral-700" }
                    }
                }

                AccordionItem {
                    index: index,
                    AccordionTrigger {
                        div { class: "flex items-center gap-3 w-full pb-2 border-b-1",

                            Checkbox {
                                // ✅ FIX: no read_only(); just convert Signal -> ReadSignal
                                checked: checked_sig,
                                on_checked_change: {
                                    let mut checked_sig = checked_sig.clone();
                                    let mut goals_state = ctx.goals_state.clone();
                                    let mut status = ctx.status.clone();

                                    Callback::new(move |state: CheckboxState| {
                                        checked_sig.set(Some(state));
                                        let new_done = matches!(state, CheckboxState::Checked);

                                        if let Some(n) = goals_state.write().find_mut(id) {
                                            set_done(n, new_done);
                                            status.set(Some("Updated completion (remember to Save).".into()));
                                        }
                                    })
                                }
                            }

                            div { class: "flex-1 text-center font-semibold",
                                "{node.title}"
                            }

                            div { class: "text-xs text-neutral-400 whitespace-nowrap",
                                "{cat} • {time_hint}"
                            }
                        }
                    }

                    AccordionContent {
                        div { class: "space-y-2 text-sm overflow-auto ",

                            div { class: "flex gap-2 flex-wrap justify-evenly",
                                button {
                                    class: "px-2 py-1 border rounded text-xs",
                                    onclick: on_open_create_sub,
                                    "+ Subgoal"
                                }

                                if !is_editing {
                                    button {
                                        class: "px-2 py-1 border rounded text-xs",
                                        onclick: {
                                            let mut editing_id = ctx.editing_id.clone();
                                            let mut draft_title = ctx.draft_title.clone();
                                            let mut draft_s = ctx.draft_s.clone();
                                            let mut draft_m = ctx.draft_m.clone();
                                            let mut draft_a = ctx.draft_a.clone();
                                            let mut draft_r = ctx.draft_r.clone();
                                            let mut draft_t = ctx.draft_t.clone();

                                            let node = node.clone();
                                            move |_| {
                                                editing_id.set(Some(id));
                                                draft_title.set(node.title.clone());
                                                draft_s.set(node.smart.specific.clone());
                                                draft_m.set(node.smart.measurable.clone());
                                                draft_a.set(node.smart.achievable.clone());
                                                draft_r.set(node.smart.relevant.clone());
                                                draft_t.set(node.smart.time_bound.clone());
                                            }
                                        },
                                        "Edit"
                                    }
                                } else {
                                    button {
                                        class: "px-2 py-1 border rounded text-xs",
                                        onclick: {
                                            let mut goals_state = ctx.goals_state.clone();
                                            let mut status = ctx.status.clone();
                                            let mut editing_id = ctx.editing_id.clone();

                                            let draft_title = ctx.draft_title.clone();
                                            let draft_s = ctx.draft_s.clone();
                                            let draft_m = ctx.draft_m.clone();
                                            let draft_a = ctx.draft_a.clone();
                                            let draft_r = ctx.draft_r.clone();
                                            let draft_t = ctx.draft_t.clone();

                                            move |_| {
                                                if let Some(n) = goals_state.write().find_mut(id) {
                                                    n.title = draft_title.read().trim().to_string();
                                                    n.smart.specific = draft_s.read().clone();
                                                    n.smart.measurable = draft_m.read().clone();
                                                    n.smart.achievable = draft_a.read().clone();
                                                    n.smart.relevant = draft_r.read().clone();
                                                    n.smart.time_bound = draft_t.read().clone();
                                                    n.touch();

                                                    editing_id.set(None);
                                                    status.set(Some("Updated goal (remember to Save).".into()));
                                                }
                                            }
                                        },
                                        "Save"
                                    }

                                    button {
                                        class: "px-2 py-1 border rounded text-xs",
                                        onclick: {
                                            let mut editing_id = ctx.editing_id.clone();
                                            move |_| editing_id.set(None)
                                        },
                                        "Cancel"
                                    }
                                }
                            }

                            if is_editing {
                                div { class: "space-y-2",
                                    label { class: "text-xs text-neutral-300", "Title" }
                                    input {
                                        class: "border px-2 py-1 w-full bg-transparent",
                                        value: "{ctx.draft_title.read()}",
                                        oninput: { let mut s = ctx.draft_title.clone(); move |e| s.set(e.value()) }
                                    }

                                    label { class: "text-xs text-neutral-300", "S — Specific" }
                                    textarea {
                                        class: "border px-2 py-1 w-full bg-transparent",
                                        rows: "2",
                                        value: "{ctx.draft_s.read()}",
                                        oninput: { let mut s = ctx.draft_s.clone(); move |e| s.set(e.value()) }
                                    }

                                    label { class: "text-xs text-neutral-300", "M — Measurable" }
                                    textarea {
                                        class: "border px-2 py-1 w-full bg-transparent",
                                        rows: "2",
                                        value: "{ctx.draft_m.read()}",
                                        oninput: { let mut s = ctx.draft_m.clone(); move |e| s.set(e.value()) }
                                    }

                                    label { class: "text-xs text-neutral-300", "A — Achievable" }
                                    textarea {
                                        class: "border px-2 py-1 w-full bg-transparent",
                                        rows: "2",
                                        value: "{ctx.draft_a.read()}",
                                        oninput: { let mut s = ctx.draft_a.clone(); move |e| s.set(e.value()) }
                                    }

                                    label { class: "text-xs text-neutral-300", "R — Relevant" }
                                    textarea {
                                        class: "border px-2 py-1 w-full bg-transparent",
                                        rows: "2",
                                        value: "{ctx.draft_r.read()}",
                                        oninput: { let mut s = ctx.draft_r.clone(); move |e| s.set(e.value()) }
                                    }

                                    label { class: "text-xs text-neutral-300", "T — Time-bound" }
                                    textarea {
                                        class: "border px-2 py-1 w-full bg-transparent",
                                        rows: "2",
                                        value: "{ctx.draft_t.read()}",
                                        oninput: { let mut s = ctx.draft_t.clone(); move |e| s.set(e.value()) }
                                    }
                                }
                            } else {
								div {
									class:"flex justify-center",
									div { class: "space-y-2 max-w-90 ",
										p { span { class: "font-extrabold", "S: " } "{node.smart.specific}" }
										p { span { class: "font-extrabold", "M: " } "{node.smart.measurable}" }
										p { span { class: "font-extrabold", "A: " } "{node.smart.achievable}" }
										p { span { class: "font-extrabold", "R: " } "{node.smart.relevant}" }
										p { span { class: "font-extrabold", "T: " } "{node.smart.time_bound}" }
									}
								}
                            }

                            if !child_items.is_empty() {
                                div { class: "pt-2 border-t border-neutral-800 space-y-2 ",
                                    for (cid, cidx) in child_items {
                                        GoalTreeItem { key: "{cid}", id: cid, depth: depth + 1, index: cidx }
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
