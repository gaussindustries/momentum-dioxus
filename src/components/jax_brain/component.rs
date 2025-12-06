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
								}
							}
						}
					}
				}
        	}
    	}
	}
}
