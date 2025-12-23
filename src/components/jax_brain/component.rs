use dioxus::prelude::*;
use strum::{EnumIter, Display, IntoEnumIterator};

use crate::components::select::{
    Select, SelectTrigger, SelectValue, SelectList,
    SelectGroup, SelectGroupLabel, SelectOption, SelectItemIndicator,
};
use crate::components::graph_generator::GraphGenerator;

use crate::models::jaxBrain::node::Graph;
use crate::models::jaxBrain::templates::dictionary::{
    DictionaryFile, build_graph_from_dictionary, layout_dictionary_graph, load_dictionary_from_path, save_dictionary_to_path
};

use crate::models::jaxBrain::templates::dictionary::{DefinitionEntry};


// Default dev path – adjust as needed
const DEFAULT_DICT_PATH: &str = "assets/data/jaxbrain/blacksLaw/ninthEd.json";

#[derive(Debug, Clone, Copy, PartialEq, EnumIter, Display)]
enum TemplateKind {
    Dictionary,
    // Timeline,
    // MindMap,
}

impl TemplateKind {
    fn label(&self) -> String {
        match self {
            TemplateKind::Dictionary => "Dictionary (Definitions)".to_string(),
        }
    }
}

#[component]
pub fn JaxBrain(
    #[props(default)]
    overview: bool,
) -> Element {
    let mut template_kind = use_signal(|| TemplateKind::Dictionary);
    let mut file_path     = use_signal(|| DEFAULT_DICT_PATH.to_string());
    let mut dict_state    = use_signal(|| None::<DictionaryFile>);
    let mut status        = use_signal(|| None::<String>);

    let graph: Option<Graph> = match *template_kind.read() {
        TemplateKind::Dictionary => {
            dict_state.read().as_ref().map(build_graph_from_dictionary)
        }
    };

    let template_options = TemplateKind::iter().enumerate().map(|(i, t)| {
        rsx! {
            SelectOption::<TemplateKind> {
                index: i,
                value: t,
                text_value: t.label(),
                { t.label() }
                SelectItemIndicator {}
            }
        }
    });

	let mut edit_open = use_signal(|| false);
	let mut edit_term_key = use_signal(|| None::<String>);

	// draft fields for the editor UI
	let mut edit_definition = use_signal(|| "".to_string());
	let mut edit_usage = use_signal(|| "".to_string());
	let mut edit_page = use_signal(|| 0_i32);
	let mut edit_time_period = use_signal(|| "".to_string());

	let on_add_term = {
    let mut dict_state = dict_state.clone();
    let mut status = status.clone();

		move |_| {
			let Some(mut dict) = dict_state.read().clone() else {
				status.set(Some("Load a dictionary first.".to_string()));
				return;
			};

			// generate unique key
			let mut i = 1;
			let mut key = format!("new_term_{i}");
			while dict.definitions.contains_key(&key) {
				i += 1;
				key = format!("new_term_{i}");
			}

			dict.definitions.insert(key.clone(), DefinitionEntry {
				found_on_page: 0,
				time_period: "".to_string(),
				definition: "".to_string(),
				usage: "".to_string(),
				examples: vec![],
				related_terms: vec![],
			});

			dict_state.set(Some(dict));
			status.set(Some(format!("Added term: {key}")));
		}
	};

	// Open editor for selected node (expects "term:xyz")
		let on_edit_selected = {
			let mut dict_state = dict_state.clone();
			let mut status = status.clone();

			let mut edit_open = edit_open.clone();
			let mut edit_term_key = edit_term_key.clone();
			let mut edit_definition = edit_definition.clone();
			let mut edit_usage = edit_usage.clone();
			let mut edit_page = edit_page.clone();
			let mut edit_time_period = edit_time_period.clone();

			move |sel: Option<String>| {
				let Some(sel_id) = sel else {
					status.set(Some("No node selected.".to_string()));
					return;
				};

				// Only edit dictionary terms for now
				let Some(term_key) = sel_id.strip_prefix("term:").map(|s| s.to_string()) else {
					status.set(Some("Select a TERM node to edit.".to_string()));
					return;
				};

				// IMPORTANT: keep the guard alive
				let dict_guard = dict_state.read();
				let Some(dict) = dict_guard.as_ref() else {
					status.set(Some("Load a dictionary first.".to_string()));
					return;
				};

				let Some(entry) = dict.definitions.get(&term_key) else {
					status.set(Some(format!("Term not found in dictionary: {term_key}")));
					return;
				};

				// populate editor draft fields
				edit_term_key.set(Some(term_key));
				edit_definition.set(entry.definition.clone());
				edit_usage.set(entry.usage.clone());
				edit_page.set(entry.found_on_page);
				edit_time_period.set(entry.time_period.clone());
				edit_open.set(true);
			}
		};

		let on_save_edit = {
		let mut dict_state = dict_state.clone();
		let mut status = status.clone();

		let mut edit_open = edit_open.clone();
		let term_key_sig = edit_term_key.clone();

		let def_sig = edit_definition.clone();
		let usage_sig = edit_usage.clone();
		let page_sig = edit_page.clone();
		let tp_sig = edit_time_period.clone();

		move |_| {
			let Some(mut dict) = dict_state.read().clone() else {
				status.set(Some("Load a dictionary first.".to_string()));
				return;
			};

			let Some(term_key) = term_key_sig.read().clone() else {
				status.set(Some("No term loaded in editor.".to_string()));
				return;
			};

			let Some(entry) = dict.definitions.get_mut(&term_key) else {
				status.set(Some(format!("Term not found: {term_key}")));
				return;
			};

			entry.definition = def_sig.read().clone();
			entry.usage = usage_sig.read().clone();
			entry.found_on_page = *page_sig.read();
			entry.time_period = tp_sig.read().clone();

			dict_state.set(Some(dict));
			edit_open.set(false);
			status.set(Some(format!("Saved edits for {term_key}")));
		}
	};

    rsx! {
        div { class: "jaxbrain-container p-4 text-secondary-color space-y-4",

            // Header
            if overview {
                h2 { class: "text-xl font-bold mb-2",
                    "JaxBrain – Overview"
                }
                p { class: "text-sm text-neutral-400",
                    "Switch to detail mode to import/edit/export JSON and view graphs."
                }
            } else {
                h2 { class: "text-xl font-bold mb-2",
                    "JaxBrain – Detail (import/edit/export JSON)"
                }
            }

            // Template selector
            div { class: "flex items-center gap-4",
                div { class: "text-sm text-neutral-300", "Template:" }

                Select::<TemplateKind> {
                    placeholder: "Select template...",

                    on_value_change: {
                        move |value: Option<TemplateKind>| {
                            if let Some(kind) = value {
                                template_kind.set(kind);
                            }
                        }
                    },

                    SelectTrigger {
                        aria_label: "Select Template",
                        width: "14rem",
                        SelectValue {}
                    }

                    SelectList { aria_label: "Template Types",
                        SelectGroup {
                            SelectGroupLabel { "Templates" }
                            { template_options }
                        }
                    }
                }
            }

            // Detail mode UI
            if !overview {
                // Path + Load/Save
                div { class: "flex items-center gap-2 mt-4",

                    input {
                        class: "border px-2 py-1 flex-1 bg-transparent",
                        value: "{file_path.read()}",
                        oninput: {
                            let mut file_path = file_path.clone();
                            move |evt| {
                                file_path.set(evt.value().to_string());
                            }
                        }
                    }

                    button {
                        class: "px-3 py-1 border rounded",
                        onclick: {
                            move |_| {
                                let path = file_path.read().clone();

                                match *template_kind.read() {
                                    TemplateKind::Dictionary => {
                                        match load_dictionary_from_path(&path) {
                                            Ok(dict) => {
                                                dict_state.set(Some(dict));
                                                status.set(Some(format!("Loaded dictionary from {}", path)));
                                            }
                                            Err(e) => {
                                                dict_state.set(None);
                                                status.set(Some(e));
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        "Load"
                    }

                    button {
                        class: "px-3 py-1 border rounded",
                        onclick: {
							move |_| {
                                let path = file_path.read().clone();

                                match *template_kind.read() {
                                    TemplateKind::Dictionary => {
                                        if let Some(dict) = dict_state.read().as_ref() {
                                            match save_dictionary_to_path(&path, dict) {
                                                Ok(()) => status.set(Some(format!("Saved dictionary to {}", path))),
                                                Err(e) => status.set(Some(e)),
                                            }
                                        } else {
                                            status.set(Some("No dictionary loaded".to_string()));
                                        }
                                    }
                                }
                            }
                        },
                        "Save"
                    }
                }

                if let Some(msg) = status.read().as_ref() {
                    p { class: "text-sm text-blue-400 mt-1", "{msg}" }
                }

                // Graph section
                // Graph section
				match graph {
					None => rsx! {
						p { class: "mt-4",
							"No data loaded yet for template {template_kind.read().label()}. Type a path and press Load."
						}
					},
					Some(graph) => {
						// normal Rust block
						let visual = layout_dictionary_graph(&graph, Some("Black's Law Dictionary (9th)"));

						// this arm must return an Element, so we return rsx! { ... }
						rsx! {
							div { class: "mt-4",
								GraphGenerator {
									graph: graph.clone(),
									visual: visual,
									title: Some("Black's Law Dictionary (9th)".to_string()),
									dict_state: dict_state,
									status: status,
									on_add_term: on_add_term,
									on_edit_selected: on_edit_selected,
								}

								if *edit_open.read() {
									div { class: "bg-neutral-900 border border-neutral-700 rounded-lg p-4 space-y-2",
										h3 { class: "font-semibold", "Edit Term" }

										if let Some(k) = edit_term_key.read().as_ref() {
											p { class: "text-xs text-neutral-400", "term: {k}" }
										}

										label { class: "text-xs text-neutral-300", "Page" }
										input {
											class: "border px-2 py-1 w-full bg-transparent",
											value: "{edit_page.read()}",
											oninput: {
												let mut edit_page = edit_page.clone();
												move |evt| {
													// naive parse
													if let Ok(v) = evt.value().parse::<i32>() {
														edit_page.set(v);
													}
												}
											}
										}

										label { class: "text-xs text-neutral-300", "Time period" }
										input {
											class: "border px-2 py-1 w-full bg-transparent",
											value: "{edit_time_period.read()}",
											oninput: {
												let mut sig = edit_time_period.clone();
												move |evt| sig.set(evt.value().to_string())
											}
										}

										label { class: "text-xs text-neutral-300", "Definition" }
										textarea {
											class: "border px-2 py-1 w-full bg-transparent",
											rows: "4",
											value: "{edit_definition.read()}",
											oninput: {
												let mut sig = edit_definition.clone();
												move |evt| sig.set(evt.value().to_string())
											}
										}

										label { class: "text-xs text-neutral-300", "Usage" }
										textarea {
											class: "border px-2 py-1 w-full bg-transparent",
											rows: "4",
											value: "{edit_usage.read()}",
											oninput: {
												let mut sig = edit_usage.clone();
												move |evt| sig.set(evt.value().to_string())
											}
										}

										div { class: "flex gap-2 pt-2",
											button {
												class: "px-3 py-1 border rounded",
												onclick: on_save_edit,
												"Save"
											}
											button {
												class: "px-3 py-1 border rounded",
												onclick: {
													let mut edit_open = edit_open.clone();
													move |_| edit_open.set(false)
												},
												"Cancel"
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
