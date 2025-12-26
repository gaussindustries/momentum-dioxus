use dioxus::prelude::*;
use std::collections::HashMap;

use crate::models::jaxBrain::node::{Graph, VisualGraph};
use crate::models::jaxBrain::templates::dictionary::DictionaryFile;

use crate::components::context_menu::{
    ContextMenu, ContextMenuTrigger, ContextMenuContent, ContextMenuItem,
};

use crate::dioxus_elements::geometry::WheelDelta;


const GRAPH_HEIGHT: i32 = 1250;
const DETAILS_WIDTH: i32 = 260;
const HOVER_RADIUS: f64 = 80.0;
const PUSH_STRENGTH: f64 = 0.4;

const SVG_PX_W: f64 = 2500.0;
const SVG_PX_H: f64 = GRAPH_HEIGHT as f64;

const ZOOM_STEP: f64 = 1.12;      // higher = faster zoom
const MIN_VB_W: f64 = 120.0;      // clamp zoom in
const MAX_VB_W: f64 = 12000.0;    // clamp zoom out


#[component]
pub fn GraphGenerator(
    graph: Graph,
    visual: VisualGraph,

    // for inline editing
    dict_state: Signal<Option<DictionaryFile>>,
    status: Signal<Option<String>>,

    on_add_term: EventHandler<()>,
    // optional: still callable from menu, but not required anymore
    on_edit_selected: EventHandler<Option<String>>,
    #[props(default)]
    on_delete_selected: Option<EventHandler<Option<String>>>,

    #[props(default)]
    title: Option<String>,
) -> Element {
    // hovered node for preview
    let mut hovered_id = use_signal(|| None::<String>);
    // pinned node for edit mode
    let mut selected_id = use_signal(|| None::<String>);
    // explicit edit mode toggle
    let mut edit_mode = use_signal(|| false);

    // Position map for edges
    let mut pos_map: HashMap<String, (f64, f64)> = HashMap::new();
    for n in &visual.nodes {
        pos_map.insert(n.id.clone(), (n.x, n.y));
    }

    // Which node is currently displayed in the panel?
    // - in edit mode: pinned selection only
    // - otherwise: hover wins, fallback to pinned selection
    let display_id: Option<String> = if *edit_mode.read() {
        selected_id.read().clone()
    } else {
        hovered_id.read().clone().or_else(|| selected_id.read().clone())
    };

    // Focus position for repulsion:
    // - if hovering, use hover
    // - else use pinned
    let focus_pos: Option<(f64, f64)> = hovered_id
        .read()
        .as_ref()
        .and_then(|hid| {
            visual.nodes.iter().find(|n| &n.id == hid).map(|n| (n.x, n.y))
        })
        .or_else(|| {
            selected_id
                .read()
                .as_ref()
                .and_then(|sid| visual.nodes.iter().find(|n| &n.id == sid).map(|n| (n.x, n.y)))
        });

    // -------- details (read-only) derived from semantic Graph --------
    let mut selected_label: Option<String> = None;
    let mut selected_kind: Option<String> = None;
    let mut selected_def: Option<String> = None;
    let mut selected_page: Option<i64> = None;
    let mut selected_usage: Option<String> = None;
    let mut selected_expansion: Option<String> = None;

    if let Some(sel_id) = display_id.as_ref() {
        if let Some(node) = graph.nodes.iter().find(|n| &n.id == sel_id) {
            selected_label = Some(node.label.clone());
            selected_kind = Some(node.kind.clone());

            if node.kind == "term" {
                selected_def = node
                    .data
                    .get("definition")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                selected_page = node.data.get("found_on_page").and_then(|v| v.as_i64());

                selected_usage = node
                    .data
                    .get("usage")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            } else if node.kind == "abbreviation" {
                selected_expansion = node
                    .data
                    .get("expansion")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }
        }
    }

    // -------- inline editor draft state (term only) --------
    let mut draft_term_key = use_signal(|| None::<String>);
    let mut draft_definition = use_signal(|| "".to_string());
    let mut draft_usage = use_signal(|| "".to_string());
    let mut draft_page = use_signal(|| 0_i32);
    let mut draft_time_period = use_signal(|| "".to_string());
    let mut draft_dirty = use_signal(|| false);
	let mut draft_term_key_new = use_signal(|| "".to_string());
	let mut draft_related_terms = use_signal(|| Vec::<String>::new());
	let mut related_input = use_signal(|| "".to_string());

    // Populate drafts ONLY when edit_mode is on (and selection exists)
	{
		let dict_state = dict_state.clone();
		let mut status = status.clone();

		let edit_mode_sig = edit_mode.clone();
		let selected_id_sig = selected_id.clone();

		let mut draft_term_key = draft_term_key.clone();
		let mut draft_term_key_new = draft_term_key_new.clone();
		let mut draft_definition = draft_definition.clone();
		let mut draft_usage = draft_usage.clone();
		let mut draft_page = draft_page.clone();
		let mut draft_time_period = draft_time_period.clone();
		let mut draft_dirty = draft_dirty.clone();

		use_effect(move || {
			// Always read *current* state inside the effect
			let editing = *edit_mode_sig.read();
			let selected_now = selected_id_sig.read().clone();

			// When not editing, clear drafts
			if !editing {
				draft_dirty.set(false);
				draft_term_key.set(None);
				draft_term_key_new.set("".to_string());
				draft_definition.set("".to_string());
				draft_usage.set("".to_string());
				draft_page.set(0);
				draft_time_period.set("".to_string());
				draft_related_terms.set(vec![]);
				return;
			}

			draft_dirty.set(false);

			let Some(sel_id) = selected_now else {
				draft_term_key.set(None);
				return;
			};

			let Some(term_key) = sel_id.strip_prefix("term:").map(|s| s.to_string()) else {
				status.set(Some("Select a TERM node to edit.".to_string()));
				draft_term_key.set(None);
				return;
			};

			let dict_guard = dict_state.read();
			let Some(dict) = dict_guard.as_ref() else {
				status.set(Some("Load a dictionary first.".to_string()));
				draft_term_key.set(None);
				return;
			};

			let Some(entry) = dict.definitions.get(&term_key) else {
				status.set(Some(format!("Term not found: {term_key}")));
				draft_term_key.set(None);
				return;
			};

			// Populate drafts
			draft_term_key.set(Some(term_key.clone()));
			draft_term_key_new.set(term_key);
			draft_definition.set(entry.definition.clone());
			draft_usage.set(entry.usage.clone());
			draft_page.set(entry.found_on_page);
			draft_time_period.set(entry.time_period.clone());
			draft_related_terms.set(entry.related_terms.clone());
		});
	}


   let on_save_inline = {
		let mut dict_state = dict_state.clone();
		let mut status = status.clone();

		let mut term_key_sig = draft_term_key.clone();
		let new_key_sig = draft_term_key_new.clone();

		let def_sig = draft_definition.clone();
		let usage_sig = draft_usage.clone();
		let page_sig = draft_page.clone();
		let tp_sig = draft_time_period.clone();

		let mut dirty_sig = draft_dirty.clone();
		let mut edit_mode = edit_mode.clone();
		let mut selected_id = selected_id.clone();

		move |_| {
			let Some(mut dict) = dict_state.read().clone() else {
				status.set(Some("Load a dictionary first.".to_string()));
				return;
			};

			let Some(old_key) = term_key_sig.read().clone() else {
				status.set(Some("No term loaded in editor.".to_string()));
				return;
			};

			let new_key = new_key_sig.read().trim().to_string();
			if new_key.is_empty() {
				status.set(Some("Term key can't be empty.".to_string()));
				return;
			}

			// Rename if changed
			if new_key != old_key {
				if dict.definitions.contains_key(&new_key) {
					status.set(Some(format!("Key already exists: {new_key}")));
					return;
				}

				let Some(entry_moved) = dict.definitions.remove(&old_key) else {
					status.set(Some(format!("Old key not found: {old_key}")));
					return;
				};

				dict.definitions.insert(new_key.clone(), entry_moved);

				// IMPORTANT: update pinned selection to new key
				selected_id.set(Some(format!("term:{new_key}")));
				term_key_sig.set(Some(new_key.clone())); // keep draft_term_key in sync
			}

			// Now write fields onto the (possibly renamed) entry
			let key = term_key_sig.read().clone().unwrap_or(new_key.clone());
			let Some(entry) = dict.definitions.get_mut(&key) else {
				status.set(Some(format!("Term not found after rename: {key}")));
				return;
			};

			entry.definition = def_sig.read().clone();
			entry.usage = usage_sig.read().clone();
			entry.found_on_page = *page_sig.read();
			entry.time_period = tp_sig.read().clone();
			entry.related_terms = draft_related_terms.read().clone();

			dict_state.set(Some(dict));
			dirty_sig.set(false);
			status.set(Some(format!("Saved: {key}")));
			edit_mode.set(false);
		}
	};

	// -------- alphabetical page starts editor --------
	let mut alpha_edit_mode = use_signal(|| false);
	let mut alpha_dirty = use_signal(|| false);
	// store as a stable list for UI (sorted)
	let mut alpha_draft = use_signal(|| Vec::<(String, i32)>::new());
	{
		let dict_state = dict_state.clone();
		let mut status = status.clone();

		let mut alpha_draft = alpha_draft.clone();
		let mut alpha_dirty = alpha_dirty.clone();

		// snapshot the "current" selection + mode for this render
		let alpha_edit_now = *alpha_edit_mode.read();
		let selected_now = selected_id.read().clone();

		use_effect(move || {
			// only load draft when alpha edit is on AND defs category is selected
			if !alpha_edit_now || selected_now.as_deref() != Some("cat:definitions") {
				alpha_dirty.set(false);
				alpha_draft.set(vec![]);
				return;
			}

			alpha_dirty.set(false);

			let guard = dict_state.read();
			let Some(dict) = guard.as_ref() else {
				status.set(Some("Load a dictionary first.".to_string()));
				return;
			};

			// build sorted list
			let mut items: Vec<(String, i32)> = dict
				.alphabetical_page_starts
				.iter()
				.map(|(k, v)| (k.clone(), *v))
				.collect();

			items.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
			alpha_draft.set(items);
		});
	}
	let on_save_alpha = {
    let mut dict_state = dict_state.clone();
    let mut status = status.clone();

    let alpha_draft = alpha_draft.clone();
    let mut alpha_dirty = alpha_dirty.clone();
    let mut alpha_edit_mode = alpha_edit_mode.clone();

    move |_| {
			let Some(mut dict) = dict_state.read().clone() else {
				status.set(Some("Load a dictionary first.".to_string()));
				return;
			};

			// write back (same data, just updated)
			let mut new_map = std::collections::BTreeMap::<String, i32>::new();
			for (k, v) in alpha_draft.read().iter() {
				new_map.insert(k.clone(), *v);
			}

			dict.alphabetical_page_starts = new_map;
			dict_state.set(Some(dict));

			alpha_dirty.set(false);
			alpha_edit_mode.set(false);
			status.set(Some("Saved alphabetical page starts.".to_string()));
		}
	};

	// --- pan/zoom (viewBox) ---
	let mut vb_x = use_signal(|| -200.0_f64);
	let mut vb_y = use_signal(|| 100.0_f64);
	let mut vb_w = use_signal(|| 1000.0_f64);
	let mut vb_h = use_signal(|| 250.0_f64);

	let mut is_panning = use_signal(|| false);
	let mut last_mouse = use_signal(|| (0.0_f64, 0.0_f64));

	// Zoom with mouse wheel (zooms around the center for simplicity)
	let on_wheel = {
		let mut vb_w = vb_w.clone();
		let mut vb_h = vb_h.clone();
		let mut vb_x = vb_x.clone();
		let mut vb_y = vb_y.clone();

		move |evt: Event<WheelData>| {
			// Dioxus doesn't give delta_y(); use delta() and pattern-match.
			let dy: f64 = match evt.delta() {
				WheelDelta::Lines(v) => v.y * 40.0,   // lines -> pixels-ish
				WheelDelta::Pixels(v) => v.y,         // already pixels
				WheelDelta::Pages(v) => v.y * 800.0,  // optional, but nice to handle
			};


			let zoom_in = dy < 0.0;
			let factor = if zoom_in { 1.0 / ZOOM_STEP } else { ZOOM_STEP };

			let old_w = *vb_w.read();
			let old_h = *vb_h.read();

			let mut new_w = (old_w * factor).clamp(MIN_VB_W, MAX_VB_W);

			// keep aspect ratio stable
			let aspect = old_h / old_w;
			let new_h = new_w * aspect;

			// zoom around center of current view
			let cx = *vb_x.read() + old_w * 0.5;
			let cy = *vb_y.read() + old_h * 0.5;

			vb_x.set(cx - new_w * 0.5);
			vb_y.set(cy - new_h * 0.5);
			vb_w.set(new_w);
			vb_h.set(new_h);
		}
	};



	let on_mouse_down = {
		let mut is_panning = is_panning.clone();
		let mut last_mouse = last_mouse.clone();
		move |evt: MouseEvent| {
			is_panning.set(true);
			last_mouse.set((evt.client_coordinates().x as f64, evt.client_coordinates().y as f64));
		}
	};

	let on_mouse_up = {
		let mut is_panning = is_panning.clone();
		move |_evt: MouseEvent| {
			is_panning.set(false);
		}
	};

	let on_mouse_leave = {
		let mut is_panning = is_panning.clone();
		move |_evt: MouseEvent| {
			is_panning.set(false);
		}
	};

	let on_mouse_move = {
		let mut vb_x = vb_x.clone();
		let mut vb_y = vb_y.clone();
		let vb_w = vb_w.clone();
		let vb_h = vb_h.clone();

		let is_panning = is_panning.clone();
		let mut last_mouse = last_mouse.clone();

		move |evt: MouseEvent| {
			if !*is_panning.read() {
				return;
			}

			let (lx, ly) = *last_mouse.read();
			let cx = evt.client_coordinates().x as f64;
			let cy = evt.client_coordinates().y as f64;

			let dx_px = cx - lx;
			let dy_px = cy - ly;

			// snapshot these BEFORE set()
			let cur_x = *vb_x.read();
			let cur_y = *vb_y.read();
			let cur_w = *vb_w.read();
			let cur_h = *vb_h.read();

			// pixel -> world scaling based on viewBox size vs svg pixel size
			let sx = cur_w / SVG_PX_W;
			let sy = cur_h / SVG_PX_H;

			// drag right -> view moves left
			vb_x.set(cur_x - dx_px * sx);
			vb_y.set(cur_y - dy_px * sy);

			last_mouse.set((cx, cy));
		}
	};
	

    rsx! {
        div {
            class: "graph-generator flex gap-4",
            style: format!("height: {}px;", GRAPH_HEIGHT),

            // ============= SVG PANEL =============
            ContextMenu {
                ContextMenuTrigger {
                    div {
                        class: "flex-1 bg-neutral-900 rounded-lg p-3 h-full",
						// pan/zoom handlers
							onwheel: on_wheel,
							onmousedown: on_mouse_down,
							onmouseup: on_mouse_up,
							onmouseleave: on_mouse_leave,
							onmousemove: on_mouse_move,

							// optional: nicer UX cursor
							style: format!(
								"border: 1px solid #444; cursor: {};",
								if *is_panning.read() { "grabbing" } else { "grab" }
							),
                        svg {
                            class: "block w-full h-full bg-[#020617] rounded-md",
							height: format!("{}", GRAPH_HEIGHT),
							width: 2500,

							view_box: format!(
								"{} {} {} {}",
								*vb_x.read(),
								*vb_y.read(),
								*vb_w.read(),
								*vb_h.read(),
							),

							

                            // edges
                            for e in &visual.edges {
                                if let (Some((x1, y1)), Some((x2, y2))) =
                                    (pos_map.get(&e.from), pos_map.get(&e.to))
                                {
                                    line {
                                        x1: format!("{x1}"),
                                        y1: format!("{y1}"),
                                        x2: format!("{x2}"),
                                        y2: format!("{y2}"),
                                        stroke: match e.kind.as_str() {
                                            "hierarchy" => "#64748b",
                                            "category"  => "#475569",
                                            "related" | "related_to" => "#334155",
                                            _ => "#475569",
                                        },
                                        "stroke-width": "1",
                                        "stroke-opacity": "0.9",
                                    }
                                }
                            }

                            // nodes
                            for vn in &visual.nodes {
                                {
                                    // highlight hovered or pinned
                                    let is_hovered = hovered_id.read().as_ref().map(|id| id == &vn.id).unwrap_or(false);
                                    let is_selected = selected_id.read().as_ref().map(|id| id == &vn.id).unwrap_or(false);
                                    let is_active = is_hovered || is_selected;

                                    let mut x = vn.x;
                                    let mut y = vn.y;

                                    // repulsion away from focus_pos (hover preferred)
                                    if let Some((fx, fy)) = focus_pos {
                                        if vn.x != fx || vn.y != fy {
                                            let dx = x - fx;
                                            let dy = y - fy;
                                            let dist_sq = dx * dx + dy * dy;

                                            if dist_sq > 0.0 {
                                                let dist = dist_sq.sqrt();
                                                if dist < HOVER_RADIUS {
                                                    let factor = (HOVER_RADIUS - dist) / HOVER_RADIUS * PUSH_STRENGTH;
                                                    let nx = dx / dist;
                                                    let ny = dy / dist;

                                                    x += nx * factor * HOVER_RADIUS;
                                                    y += ny * factor * HOVER_RADIUS;
                                                }
                                            }
                                        }
                                    }

                                    let base_r: f64 = match vn.kind.as_str() {
                                        "root"     => 24.0,
                                        "category" => 18.0,
                                        _          => 14.0,
                                    };
                                    let base_font: f64 = match vn.kind.as_str() {
                                        "root"     => 14.0,
                                        "category" => 11.0,
                                        _          => 9.0,
                                    };

                                    let r = if is_active { base_r * 1.25 } else { base_r };
                                    let font_size = if is_active { base_font * 1.15 } else { base_font };
                                    let ring_r = r * 1.3;

                                    rsx! {
                                        g {
                                            key: "{vn.id}",

                                            // hover drives preview (NOT edit)
                                            onmouseenter: {
                                                let id = vn.id.clone();
                                                let mut hovered = hovered_id.clone();
                                                move |_| hovered.set(Some(id.clone()))
                                            },
                                            onmouseleave: {
                                                let mut hovered = hovered_id.clone();
                                                move |_| hovered.set(None)
                                            },

                                            // click "pins" selection (still not edit mode)
                                            onclick: {
                                                let id = vn.id.clone();
                                                let mut selected = selected_id.clone();
                                                let mut edit_mode = edit_mode.clone();
                                                move |_| {
                                                    selected.set(Some(id.clone()));
                                                    // clicking a node exits edit mode (so you don't accidentally keep editing old node)
                                                    edit_mode.set(false);
                                                }
                                            },

                                            if is_active {
                                                circle {
                                                    cx: format!("{x}"),
                                                    cy: format!("{y}"),
                                                    r: format!("{ring_r}"),
                                                    stroke: if is_selected { "#38bdf8" } else { "#475569" },
                                                    "stroke-width": "3",
                                                    fill: "none",
                                                }
                                            }

                                            circle {
                                                cx: format!("{x}"),
                                                cy: format!("{y}"),
                                                r: format!("{r}"),
                                                fill: match vn.kind.as_str() {
                                                    "root"         => "#0ea5e9",
                                                    "category"     => "#6366f1",
                                                    "abbreviation" => "#f59e0b",
                                                    _              => "#22c55e",
                                                },
                                                stroke: "#020617",
                                                "stroke-width": "1.5",
                                            }

                                            text {
                                                x: format!("{x}"),
                                                y: format!("{}", y + 4.0),
                                                "text-anchor": "middle",
                                                "font-size": format!("{font_size}"),
                                                fill: "#e5e7eb",
                                                "{vn.label}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                ContextMenuContent {
                    ContextMenuItem {
                        value: "add_term",
                        index: 0usize,
                        on_select: move |_| on_add_term.call(()),
                        "Add term"
                    }

                    // keep if you want it (but your panel is the main edit flow now)
                    ContextMenuItem {
                        value: "edit_selected",
                        index: 1usize,
                        on_select: {
                            let selected_id = selected_id.clone();
                            move |_| on_edit_selected.call(selected_id.read().clone())
                        },
                        "Edit selected (legacy)"
                    }

                    if let Some(on_delete_selected) = on_delete_selected {
                        ContextMenuItem {
                            value: "delete_selected",
                            index: 2usize,
                            on_select: {
                                let selected_id = selected_id.clone();
                                move |_| on_delete_selected.call(selected_id.read().clone())
                            },
                            "Delete selected"
                        }
                    }
                }
            }

            // ============= DETAILS PANEL =============
            div {
                class: "bg-neutral-900 rounded-lg p-4 text-sm space-y-2 overflow-y-auto",
                style: format!("width: {}px; height: 100%;", DETAILS_WIDTH),

                h3 { class: "font-semibold text-lg mb-1",
                    if *edit_mode.read() { "Node Details — Edit" } else { "Node Details" }
                }

                // show something even when nothing is hovered
                match display_id.as_ref() {
                    None => rsx! {
                        p { class: "text-neutral-500",
                            "Hover a node to preview details. Click a node to pin it."
                        }
                    },
                    Some(id) => rsx! {
                        // debug / sanity
                        p { class: "text-xs text-neutral-500", "id: {id}" }

                        // If term node, show Edit button in read-only mode
                        if !*edit_mode.read() && id.starts_with("term:") {
                            button {
                                class: "px-2 py-1 border rounded text-xs",
                                onclick: {
                                    let mut selected = selected_id.clone();
                                    let mut edit_mode = edit_mode.clone();
                                    let id = id.clone();
                                    move |_| {
                                        selected.set(Some(id.clone())); // pin the hovered node
                                        edit_mode.set(true);            // enter edit mode
                                    }
                                },
                                "Edit"
                            }
                        }

                        // EDIT MODE (term only)
                        if *edit_mode.read() && selected_id.read().as_ref().map(|s| s.starts_with("term:")).unwrap_or(false) {
                            div { class: "flex items-center justify-between gap-2 pt-2",
                                p { class: "text-xs text-neutral-400",
                                    if let Some(k) = draft_term_key.read().as_ref() { "term: {k}" } else { "term: <none>" }
                                }
                                div { class: "flex gap-2",
                                    button {
                                        class: if *draft_dirty.read() { "px-2 py-1 border rounded text-xs" } else { "px-2 py-1 border rounded text-xs opacity-50" },
                                        disabled: !*draft_dirty.read(),
                                        onclick: on_save_inline,
                                        "Save"
                                    }
                                    button {
                                        class: "px-2 py-1 border rounded text-xs",
                                        onclick: {
                                            let mut edit_mode = edit_mode.clone();
                                            move |_| edit_mode.set(false)
                                        },
                                        "Cancel"
                                    }
                                }
                            }
							label { class: "text-xs text-neutral-300", "Term key" }
							input {
								class: "border px-2 py-1 w-full bg-transparent",
								value: "{draft_term_key_new.read()}",
								oninput: {
									let mut sig = draft_term_key_new.clone();
									let mut dirty = draft_dirty.clone();
									move |evt| { sig.set(evt.value().to_string()); dirty.set(true); }
								}
							}

                            label { class: "text-xs text-neutral-300", "Page" }
                            input {
                                class: "border px-2 py-1 w-full bg-transparent",
                                value: "{draft_page.read()}",
                                oninput: {
                                    let mut draft_page = draft_page.clone();
                                    let mut draft_dirty = draft_dirty.clone();
                                    move |evt| {
                                        if let Ok(v) = evt.value().parse::<i32>() {
                                            draft_page.set(v);
                                            draft_dirty.set(true);
                                        }
                                    }
                                }
                            }

                            label { class: "text-xs text-neutral-300", "Time period" }
                            input {
                                class: "border px-2 py-1 w-full bg-transparent",
                                value: "{draft_time_period.read()}",
                                oninput: {
                                    let mut sig = draft_time_period.clone();
                                    let mut dirty = draft_dirty.clone();
                                    move |evt| { sig.set(evt.value().to_string()); dirty.set(true); }
                                }
                            }

                            label { class: "text-xs text-neutral-300", "Definition" }
                            textarea {
                                class: "border px-2 py-1 w-full bg-transparent",
                                rows: "6",
                                value: "{draft_definition.read()}",
                                oninput: {
                                    let mut sig = draft_definition.clone();
                                    let mut dirty = draft_dirty.clone();
                                    move |evt| { sig.set(evt.value().to_string()); dirty.set(true); }
                                }
                            }

                            label { class: "text-xs text-neutral-300", "Usage" }
                            textarea {
                                class: "border px-2 py-1 w-full bg-transparent",
                                rows: "4",
                                value: "{draft_usage.read()}",
                                oninput: {
                                    let mut sig = draft_usage.clone();
                                    let mut dirty = draft_dirty.clone();
                                    move |evt| { sig.set(evt.value().to_string()); dirty.set(true); }
                                }
                            }
							label { class: "text-xs text-neutral-300", "Related notable words" }

							// Existing terms
							div { class: "flex flex-wrap gap-2",
								for (i, term) in draft_related_terms.read().iter().cloned().enumerate() {
									div { class: "px-2 py-1 rounded border border-neutral-700 text-xs flex items-center gap-2",
										span { "{term}" }
										button {
											class: "opacity-70 hover:opacity-100",
											onclick: {
												let mut draft_related_terms = draft_related_terms.clone();
												let mut draft_dirty = draft_dirty.clone();
												move |_| {
													let mut v = draft_related_terms.read().clone();
													if i < v.len() {
														v.remove(i);
														draft_related_terms.set(v);
														draft_dirty.set(true);
													}
												}
											},
											"✕"
										}
									}
								}
							}

							// Add box
							div { class: "flex gap-2",
								input {
									class: "border px-2 py-1 w-full bg-transparent text-xs",
									placeholder: "Add (comma separated ok)…",
									value: "{related_input.read()}",
									oninput: {
										let mut related_input = related_input.clone();
										move |evt| related_input.set(evt.value().to_string())
									}
								}
								button {
									class: "px-2 py-1 border rounded text-xs",
									onclick: {
										let mut related_input = related_input.clone();
										let mut draft_related_terms = draft_related_terms.clone();
										let mut draft_dirty = draft_dirty.clone();
										move |_| {
											let raw = related_input.read().trim().to_string();
											if raw.is_empty() { return; }

											// split by commas if user pasted a bunch
											let parts = raw
												.split(',')
												.map(|s| s.trim())
												.filter(|s| !s.is_empty())
												.map(|s| s.to_string())
												.collect::<Vec<_>>();

											if parts.is_empty() { return; }

											let mut v = draft_related_terms.read().clone();
											for p in parts {
												// avoid duplicates (case-insensitive)
												let exists = v.iter().any(|x| x.eq_ignore_ascii_case(&p));
												if !exists {
													v.push(p);
												}
											}
											draft_related_terms.set(v);
											related_input.set("".to_string());
											draft_dirty.set(true);
										}
									},
									"Add"
								}
							}
						} else if id == "cat:definitions" {
								p { class: "text-neutral-300",
									span { class: "font-semibold", "Node: " }
									"Definitions"
								}

								// read mode
								if !*alpha_edit_mode.read() {
									button {
										class: "px-2 py-1 border rounded text-xs",
										onclick: {
											let mut selected = selected_id.clone();
											let mut alpha_edit_mode = alpha_edit_mode.clone();
											move |_| {
												selected.set(Some("cat:definitions".to_string())); // pin it
												alpha_edit_mode.set(true);
											}
										},
										"Edit alphabetical page starts"
									}

									div { class: "mt-2 space-y-1",
										for (k, v) in alpha_draft.read().iter() {
											div { class: "flex justify-between text-xs text-neutral-300",
												span { "{k}" }
												span { "{v}" }
											}
										}
									}
								} else {
									// edit mode
									div { class: "flex items-center justify-between gap-2 pt-2",
										h4 { class: "font-semibold", "Alphabetical page starts" }
										div { class: "flex gap-2",
											button {
												class: if *alpha_dirty.read() {
													"px-2 py-1 border rounded text-xs"
												} else {
													"px-2 py-1 border rounded text-xs opacity-50"
												},
												disabled: !*alpha_dirty.read(),
												onclick: on_save_alpha,
												"Save"
											}
											button {
												class: "px-2 py-1 border rounded text-xs",
												onclick: {
													let mut alpha_edit_mode = alpha_edit_mode.clone();
													move |_| alpha_edit_mode.set(false)
												},
												"Cancel"
											}
										}
									}

									div { class: "mt-2 space-y-2",
										for (idx, (k, v)) in alpha_draft.read().iter().cloned().enumerate() {
											div { class: "flex items-center gap-2",
												input {
													class: "border px-2 py-1 w-12 bg-transparent text-xs",
													value: "{k}",
													disabled: true,
												}
												input {
													class: "border px-2 py-1 flex-1 bg-transparent text-xs",
													value: "{v}",
													oninput: {
														let mut alpha_draft = alpha_draft.clone();
														let mut alpha_dirty = alpha_dirty.clone();
														move |evt| {
															if let Ok(num) = evt.value().parse::<i32>() {
																let mut vec = alpha_draft.read().clone();
																if let Some(item) = vec.get_mut(idx) {
																	item.1 = num;
																	alpha_draft.set(vec);
																	alpha_dirty.set(true);
																}
															}
														}
													}
												}
											}
										}
									}
								}
							} else {
                            // READ-ONLY MODE (any node)
                            if let Some(label) = selected_label.clone() {
                                p { class: "text-neutral-300",
                                    span { class: "font-semibold", "Label: " }
                                    "{label}"
                                }
                            }

                            if let Some(kind) = selected_kind.clone() {
                                p { span { class: "font-semibold", "Kind: " } "{kind}" }
                            }

                            if let Some(def) = selected_def.clone() {
                                if !def.is_empty() {
                                    div {
                                        p { class: "font-semibold mt-2 mb-1", "Definition" }
                                        p { class: "text-neutral-200 leading-snug", "{def}" }
                                    }
                                }
                            }

                            if let Some(page) = selected_page {
                                if page >= 0 {
                                    p { span { class: "font-semibold", "Page: " } "{page}" }
                                }
                            }

                            if let Some(usage) = selected_usage.clone() {
                                if !usage.is_empty() {
                                    div {
                                        p { class: "font-semibold mt-2 mb-1", "Usage" }
                                        p { class: "text-neutral-300 leading-snug", "{usage}" }
                                    }
                                }
                            }

                            if let Some(exp) = selected_expansion.clone() {
                                if !exp.is_empty() {
                                    div {
                                        p { class: "font-semibold mt-2 mb-1", "Expansion" }
                                        p { class: "text-neutral-200 leading-snug", "{exp}" }
                                    }
                                }
                            }
                        }
                    }
                }
				
                p { class: "text-xs text-neutral-500 pt-2 border-t border-neutral-800 mt-3",
                    "Nodes: {graph.nodes.len()}, edges: {graph.edges.len()}"
                }
            }
        }
    }
}
