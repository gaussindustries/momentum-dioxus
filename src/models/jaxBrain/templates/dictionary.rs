use serde::{Deserialize, Serialize};
use std::collections::BTreeMap; //hash map YUCK
use std::fs;

use crate::models::jaxBrain::node::*;

#[derive(Serialize, Deserialize, Clone)]
pub struct DictionaryFile {
    #[serde(rename = "alphabetical page starts")]
    pub alphabetical_page_starts: BTreeMap<String, i32>,

    pub abbreviations: BTreeMap<String, String>,

    pub definitions: BTreeMap<String, DefinitionEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DefinitionEntry {
    #[serde(rename = "found on page")]
    pub found_on_page: i32,

    #[serde(rename = "time period")]
    pub time_period: String,

    #[serde(rename = "definition")]
    pub definition: String,

    #[serde(rename = "usage")]
    pub usage: String,

    #[serde(rename = "e.g.")]
    pub examples: Vec<String>,

    #[serde(rename = "related notable words")]
    pub related_terms: Vec<String>,
}

pub fn build_graph_from_dictionary(raw: &DictionaryFile) -> Graph {
    use serde_json::json;

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    // --- Definitions: create term nodes ---
    for (term, def) in &raw.definitions {
        let id = format!("term:{term}");
        nodes.push(Node {
            id: id.clone(),
            label: term.clone(),
            kind: "term".into(),
            data: json!({
                "found_on_page": def.found_on_page,
                "time_period": def.time_period,
                "definition": def.definition,
                "usage": def.usage,
                "examples": def.examples,
                "related_terms": def.related_terms,
            }),
        });

        // related_to edges (term -> related term)
        for rel in &def.related_terms {
            edges.push(Edge {
                id: format!("edge:{term}->{}", rel),
                from: id.clone(),
                to: format!("term:{rel}"),
                relation: "related_to".into(),
                data: json!({}),
            });
        }
    }

    // --- Abbreviations: create abbreviation nodes ---
    for (abbr, full) in &raw.abbreviations {
        let id = format!("abbr:{abbr}");
        nodes.push(Node {
            id: id.clone(),
            label: abbr.clone(),
            kind: "abbreviation".into(),
            data: json!({ "expansion": full }),
        });
    }

    // --- NEW: create alphabetical bucket nodes (A-Z) under definitions ---
    // We'll key buckets by lowercase letter for stable IDs: alpha:a, alpha:b, ...
    // Page start pulled from raw.alphabetical_page_starts if present; else 0.
    for b in b'a'..=b'z' {
        let letter = (b as char).to_string(); // "a".."z"
        let page_start = raw
            .alphabetical_page_starts
            .get(&letter)
            .copied()
            .unwrap_or(0);

        nodes.push(Node {
            id: format!("alpha:{letter}"),
            label: letter.to_uppercase(),
            kind: "alpha".into(),
            data: json!({
                "letter": letter,
                "page_start": page_start,
            }),
        });
    }

    // --- NEW: connect each term to its alphabetical bucket ---
    for term in raw.definitions.keys() {
        let first = term
            .chars()
            .find(|c| c.is_ascii_alphabetic())
            .map(|c| c.to_ascii_lowercase())
            .unwrap_or('~'); // non-letter bucket (optional)

        if ('a'..='z').contains(&first) {
            let letter = first.to_string();
            edges.push(Edge {
                id: format!("edge:alpha:{letter}->term:{term}"),
                from: format!("alpha:{letter}"),
                to: format!("term:{term}"),
                relation: "contains".into(),
                data: json!({}),
            });
        }
    }

    Graph { nodes, edges }
}


// --------- IO helpers for import/export ----------

pub fn load_dictionary_from_path(path: &str) -> Result<DictionaryFile, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path, e))?;
    serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse JSON at {}: {}", path, e))
}

pub fn save_dictionary_to_path(path: &str, dict: &DictionaryFile) -> Result<(), String> {
    let text = serde_json::to_string_pretty(dict)
        .map_err(|e| format!("Failed to serialize dictionary: {}", e))?;
    fs::write(path, text)
        .map_err(|e| format!("Failed to write {}: {}", path, e))
}

// --------- helpers for graph generation ----------

pub fn layout_dictionary_graph(graph: &Graph, title: Option<&str>) -> VisualGraph {
    // Basic geometry
    let center_x = 300.0_f64;
    let center_y = 250.0_f64;
    let cat_radius = 250.0_f64;

    // Radii for rings
    let alpha_radius = 205.0_f64; // ring for A-Z around Definitions
    let leaf_radius  = 55.0_f64; // ring for terms around each letter bucket

    let defs_center = (center_x - cat_radius, center_y);
    let abbr_center = (center_x + cat_radius, center_y);

    // Split by kind
    let term_nodes: Vec<_> = graph.nodes.iter().filter(|n| n.kind == "term").cloned().collect();
    let abbr_nodes: Vec<_> = graph.nodes.iter().filter(|n| n.kind == "abbreviation").cloned().collect();
    let alpha_nodes: Vec<_> = graph.nodes.iter().filter(|n| n.kind == "alpha").cloned().collect();

    let mut vnodes: Vec<VisualNode> = Vec::new();
    let mut vedges: Vec<VisualEdge> = Vec::new();

    // ----- master node -----
    let master_id    = "master:dictionary".to_string();
    let master_label = title.unwrap_or("Dictionary Collection").to_string();

    vnodes.push(VisualNode {
        id: master_id.clone(),
        label: master_label,
        kind: "root".into(),
        x: center_x,
        y: center_y,
    });

    // ----- category: Definitions -----
    let defs_id = "cat:definitions".to_string();
    vnodes.push(VisualNode {
        id: defs_id.clone(),
        label: "Definitions".into(),
        kind: "category".into(),
        x: defs_center.0,
        y: defs_center.1,
    });
    vedges.push(VisualEdge {
        from: master_id.clone(),
        to: defs_id.clone(),
        kind: "hierarchy".into(),
    });

    // ----- category: Abbreviations -----
    let abbr_id = "cat:abbreviations".to_string();
    vnodes.push(VisualNode {
        id: abbr_id.clone(),
        label: "Abbreviations".into(),
        kind: "category".into(),
        x: abbr_center.0,
        y: abbr_center.1,
    });
    vedges.push(VisualEdge {
        from: master_id.clone(),
        to: abbr_id.clone(),
        kind: "hierarchy".into(),
    });

    // ----- NEW: alpha nodes around Definitions -----
    // Sort alpha nodes by label (A..Z) so they form a clean ring
    let mut alpha_sorted = alpha_nodes;
    alpha_sorted.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));

    for (i, node) in alpha_sorted.iter().enumerate() {
        let n = alpha_sorted.len().max(1);
        let angle = (i as f64) / (n as f64) * std::f64::consts::TAU;

        let x = defs_center.0 + alpha_radius * angle.cos();
        let y = defs_center.1 + alpha_radius * angle.sin();

        vnodes.push(VisualNode {
            id: node.id.clone(),
            label: node.label.clone(), // "A".."Z"
            kind: "alpha".into(),
            x,
            y,
        });

        // defs -> alpha
        vedges.push(VisualEdge {
            from: defs_id.clone(),
            to: node.id.clone(),
            kind: "category".into(),
        });
    }

    // ----- Abbr nodes around Abbreviations (same as before) -----
    let abbr_limit = abbr_nodes.len().min(40);
    for (i, node) in abbr_nodes.iter().take(abbr_limit).enumerate() {
        let angle = (i as f64) / (abbr_limit as f64).max(1.0) * std::f64::consts::TAU;
        let x = abbr_center.0 + 90.0 * angle.cos();
        let y = abbr_center.1 + 90.0 * angle.sin();

        vnodes.push(VisualNode {
            id: node.id.clone(),
            label: node.label.clone(),
            kind: node.kind.clone(),
            x,
            y,
        });

        vedges.push(VisualEdge {
            from: abbr_id.clone(),
            to: node.id.clone(),
            kind: "category".into(),
        });
    }

    // ----- OPTIONAL: place SOME term nodes around their alpha bucket centers -----
    // This keeps the graph readable. The semantic edges still include all terms.
    // Limit per-letter so you don’t explode the SVG.
    let per_letter_limit: usize = 6;

    // Build lookup: alpha_id -> its (x,y)
    let mut alpha_pos: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    for vn in &vnodes {
        if vn.kind == "alpha" {
            alpha_pos.insert(vn.id.clone(), (vn.x, vn.y));
        }
    }

    // Collect contains-edges alpha:* -> term:*
    // Then place up to N terms around each alpha node.
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for e in &graph.edges {
        if e.relation == "contains" && e.from.starts_with("alpha:") && e.to.starts_with("term:") {
            grouped.entry(e.from.clone()).or_default().push(e.to.clone());
        }
    }

    for (alpha_id, terms) in grouped {
        let Some((ax, ay)) = alpha_pos.get(&alpha_id).copied() else { continue; };

        // stable-ish order
        let mut terms = terms;
        terms.sort();

        let limit = terms.len().min(per_letter_limit);
        for (i, term_id) in terms.into_iter().take(limit).enumerate() {
            let angle = (i as f64) / (limit as f64).max(1.0) * std::f64::consts::TAU;
            let x = ax + leaf_radius * angle.cos();
            let y = ay + leaf_radius * angle.sin();

            // Find the term node label
            let label = graph
                .nodes
                .iter()
                .find(|n| n.id == term_id)
                .map(|n| n.label.clone())
                .unwrap_or_else(|| term_id.clone());

            vnodes.push(VisualNode {
                id: term_id.clone(),
                label,
                kind: "term".into(),
                x,
                y,
            });

            // alpha -> term (visual)
            vedges.push(VisualEdge {
                from: alpha_id.clone(),
                to: term_id.clone(),
                kind: "contains".into(),
            });
        }
    }

    // ----- include ALL semantic edges (related_to, contains, etc.) as visual edges -----
    for edge in &graph.edges {
        vedges.push(VisualEdge {
            from: edge.from.clone(),
            to: edge.to.clone(),
            kind: edge.relation.clone(),
        });
    }

    VisualGraph { nodes: vnodes, edges: vedges }
}

