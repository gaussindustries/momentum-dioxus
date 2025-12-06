use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

use crate::models::jaxBrain::node::*;

#[derive(Serialize, Deserialize, Clone)]
pub struct DictionaryFile {
    #[serde(rename = "alphabetical page starts")]
    pub alphabetical_page_starts: HashMap<String, i32>,

    pub abbreviations: HashMap<String, String>,

    pub definitions: HashMap<String, DefinitionEntry>,
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

    for (term, def) in &raw.definitions {
        let id = format!("term:{}", term);
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
            }),
        });

        for rel in &def.related_terms {
            let target_id = format!("term:{}", rel);
            edges.push(Edge {
                id: format!("edge:{}->{}", term, rel),
                from: id.clone(),
                to: target_id,
                relation: "related_to".into(),
                data: json!({}),
            });
        }
    }

    for (abbr, full) in &raw.abbreviations {
        let id = format!("abbr:{}", abbr);
        nodes.push(Node {
            id: id.clone(),
            label: abbr.clone(),
            kind: "abbreviation".into(),
            data: json!({ "expansion": full }),
        });
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

pub fn layout_dictionary_graph(
    graph: &Graph,
    title: Option<&str>,
) -> VisualGraph {
    // Basic geometry
    let center_x = 300.0_f64;
    let center_y = 250.0_f64;
    let cat_radius = 150.0_f64;
    let leaf_radius = 90.0_f64;

    let defs_center = (center_x - cat_radius, center_y);
    let abbr_center = (center_x + cat_radius, center_y);

    // Split by kind
    let term_nodes: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == "term")
        .cloned()
        .collect();

    let abbr_nodes: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == "abbreviation")
        .cloned()
        .collect();

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

    // ----- term nodes around Definitions -----
    let term_limit = term_nodes.len().min(40);
    for (i, node) in term_nodes.iter().take(term_limit).enumerate() {
        let angle = (i as f64) / (term_limit as f64).max(1.0) * std::f64::consts::TAU;
        let x = defs_center.0 + leaf_radius * angle.cos();
        let y = defs_center.1 + leaf_radius * angle.sin();

        vnodes.push(VisualNode {
            id: node.id.clone(),
            label: node.label.clone(),
            kind: node.kind.clone(),
            x,
            y,
        });

        vedges.push(VisualEdge {
            from: defs_id.clone(),
            to: node.id.clone(),
            kind: "category".into(),
        });
    }

    // ----- abbr nodes around Abbreviations -----
    let abbr_limit = abbr_nodes.len().min(40);
    for (i, node) in abbr_nodes.iter().take(abbr_limit).enumerate() {
        let angle = (i as f64) / (abbr_limit as f64).max(1.0) * std::f64::consts::TAU;
        let x = abbr_center.0 + leaf_radius * angle.cos();
        let y = abbr_center.1 + leaf_radius * angle.sin();

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

    // ----- real “related_to” edges from Graph -----
    // (we still render them in the visual graph)
    for edge in &graph.edges {
        vedges.push(VisualEdge {
            from: edge.from.clone(),
            to: edge.to.clone(),
            kind: edge.relation.clone(), // will usually be "related_to"
        });
    }

    VisualGraph {
        nodes: vnodes,
        edges: vedges,
    }
}
