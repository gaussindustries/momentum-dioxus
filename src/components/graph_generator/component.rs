use dioxus::prelude::*;
use std::collections::HashMap;

use crate::models::jaxBrain::node::{Graph, VisualGraph};

const GRAPH_HEIGHT: i32 = 1250;
const DETAILS_WIDTH: i32 = 260;
const HOVER_RADIUS: f64 = 80.0;     // how far the “repulsion” reaches
const PUSH_STRENGTH: f64 = 0.4;     // how strongly neighbors get pushed

#[component]
pub fn GraphGenerator(
    graph: Graph,
    visual: VisualGraph,
    #[props(default)]
    title: Option<String>,
) -> Element {
    let mut selected_id = use_signal(|| None::<String>);

    // Position map for edge drawing
    let mut pos_map: HashMap<String, (f64, f64)> = HashMap::new();
    for n in &visual.nodes {
        pos_map.insert(n.id.clone(), (n.x, n.y));
    }

    let selected_snapshot = selected_id.read().clone();
	
	// Get hovered/selected node position, if any
    let focus_pos: Option<(f64, f64)> = selected_snapshot
        .as_ref()
        .and_then(|sel_id| {
            visual
                .nodes
                .iter()
                .find(|n| &n.id == sel_id)
                .map(|n| (n.x, n.y))
        });
    // --- build details for selected node, using semantic Graph ---
    let mut selected_label: Option<String> = None;
    let mut selected_kind: Option<String> = None;
    let mut selected_def: Option<String> = None;
    let mut selected_page: Option<i64> = None;
    let mut selected_usage: Option<String> = None;
    let mut selected_expansion: Option<String> = None;

    if let Some(sel_id) = selected_snapshot.as_ref() {
        if let Some(node) = graph.nodes.iter().find(|n| &n.id == sel_id) {
            selected_label = Some(node.label.clone());
            selected_kind = Some(node.kind.clone());

            if node.kind == "term" {
                selected_def = node
                    .data
                    .get("definition")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                selected_page = node
                    .data
                    .get("found_on_page")
                    .and_then(|v| v.as_i64());

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

    rsx! {
        // Row container: SVG left, details right.
        div {
            class: "graph-generator flex gap-4",
            // single source of truth for height
            style: format!("height: {}px;", GRAPH_HEIGHT),

            // ============= SVG PANEL =============
            div {
                class: "flex-1 bg-neutral-900 rounded-lg p-3 h-full",

                svg {
                    class: "block w-full h-full bg-[#020617] rounded-md",
					height: format!("{}", GRAPH_HEIGHT),
					width: 2500,
                    view_box: "-200 100 1000 250",
                    style: "border: 1px solid #444;",

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
									let is_selected = selected_snapshot
									.as_ref()
									.map(|id| id == &vn.id)
									.unwrap_or(false);

									// Base position
									let mut x = vn.x;
									let mut y = vn.y;

									// If there is a focused node, push neighbors away slightly
									if let (Some((fx, fy)), Some(ref sel_id)) = (focus_pos, selected_snapshot.as_ref()) {
										if &vn.id != *sel_id {
											let dx = x - fx;
											let dy = y - fy;
											let dist_sq = dx * dx + dy * dy;

											if dist_sq > 0.0 {
												let dist = dist_sq.sqrt();
												if dist < HOVER_RADIUS {
													// push strength scales with closeness
													let factor = (HOVER_RADIUS - dist) / HOVER_RADIUS * PUSH_STRENGTH;
													let nx = dx / dist;
													let ny = dy / dist;

													x += nx * factor * HOVER_RADIUS;
													y += ny * factor * HOVER_RADIUS;
												}
											}
										}
									}

									// base radius & font
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

									// hovered node gets a scale-up
									let r = if is_selected { base_r * 1.3 } else { base_r };
									let font_size = if is_selected { base_font * 1.2 } else { base_font };
									let ring_r = r * 1.3;

									rsx!{
										g {
											key: "{vn.id}",

											onclick: {
												let id = vn.id.clone();
												let mut selected = selected_id.clone();
												move |_| selected.set(Some(id.clone()))
											},
											onmouseenter: {
												let id = vn.id.clone();
												let mut selected = selected_id.clone();
												move |_| selected.set(Some(id.clone()))
											},

											// selection / hover ring
											if is_selected {
												circle {
													cx: format!("{x}"),
													cy: format!("{y}"),
													r: format!("{ring_r}"),
													stroke: "#38bdf8",
													"stroke-width": "3",
													fill: "none",
												}
											}

											// main dot
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

            // ============= DETAILS PANEL =============
            div {
                class: "bg-neutral-900 rounded-lg p-4 text-sm space-y-2 overflow-y-auto",
                // fixed width, same height as container
                style: format!("width: {}px; height: 100%;", DETAILS_WIDTH),

                h3 { class: "font-semibold text-lg mb-1", "Node Details" }

                match selected_label {
                    None => rsx! {
                        p { class: "text-neutral-500",
                            "Hover or click a node to view its details."
                        }
                    },
                    Some(label) => rsx! {
                        p { class: "text-neutral-300",
                            span { class: "font-semibold", "Label: " }
                            "{label}"
                        }

                        if let Some(kind) = selected_kind.clone() {
                            p {
                                span { class: "font-semibold", "Kind: " }
                                "{kind}"
                            }
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
                                p {
                                    span { class: "font-semibold", "Page: " }
                                    "{page}"
                                }
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

                        if let Some(expansion) = selected_expansion.clone() {
                            if !expansion.is_empty() {
                                div {
                                    p { class: "font-semibold mt-2 mb-1", "Expansion" }
                                    p { class: "text-neutral-200 leading-snug", "{expansion}" }
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
