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
/// Layout-dependent radii config.
/// Keep this small and explicit so each layout can pick different sizes safely.
#[derive(Clone, Copy, Debug)]
pub struct LayoutRadii {
    pub root: f64,
    pub category: f64,
    pub alpha: f64,
    pub term: f64,
    pub abbreviation: f64,

    /// Extra space you want between circles (and between rings) to avoid visual overlap.
    pub padding: f64,

    /// Extra “spread” you want between master and categories (beyond just radii+padding).
    pub category_extra_offset: f64,

    /// Base ring radii (optional). If None, we compute something conservative from node radii.
    pub alpha_ring_radius: Option<f64>,
    pub leaf_ring_radius: Option<f64>,
    pub abbr_ring_radius: Option<f64>,
}

impl LayoutRadii {
    pub const fn roomy() -> Self {
        Self {
            root: 400.0,          // NOTE: this should match what you draw in the UI for "root"
            category: 180.0,
            alpha: 15.0,
            term: 14.0,
            abbreviation: 14.0,
            padding: 35.0,
            category_extra_offset: 40.0,
            alpha_ring_radius: None,
            leaf_ring_radius: None,
            abbr_ring_radius: None,
        }
    }

    pub const fn compact() -> Self {
        Self {
            root: 70.0,
            category: 16.0,
            alpha: 12.0,
            term: 12.0,
            abbreviation: 12.0,
            padding: 24.0,
            category_extra_offset: 25.0,
            alpha_ring_radius: None,
            leaf_ring_radius: None,
            abbr_ring_radius: None,
        }
    }

    #[inline]
    pub fn radius_for(&self, kind: &str) -> f64 {
        match kind {
            "root" => self.root,
            "category" => self.category,
            "alpha" => self.alpha,
            "term" => self.term,
            "abbreviation" => self.abbreviation,
            _ => self.term,
        }
    }

    /// Compute a safe default ring radius so alpha nodes don't overlap the defs category.
    #[inline]
    pub fn alpha_ring_radius_default(&self) -> f64 {
        // category circle + padding + alpha circle + some breathing room
        self.category + self.padding + self.alpha + 60.0
    }

    /// Compute a safe default leaf ring radius so terms don't overlap the alpha circle.
    #[inline]
    pub fn leaf_ring_radius_default(&self) -> f64 {
        self.alpha + self.padding + self.term + 30.0
    }

    /// Compute a safe default abbr ring radius around the Abbreviations category.
    #[inline]
    pub fn abbr_ring_radius_default(&self) -> f64 {
        self.category + self.padding + self.abbreviation + 40.0
    }
}

/// Convenience wrapper if you still want a free function.
/// Prefer passing LayoutRadii into layout_dictionary_graph though.
pub fn radius_for(kind: &str) -> f64 {
    LayoutRadii::roomy().radius_for(kind)
}

// --------- helpers for graph generation ----------

pub fn layout_dictionary_graph(graph: &Graph, title: Option<&str>) -> VisualGraph {
    // Pick a layout profile here (or pass it as a parameter if you want runtime selection)
    let radii = LayoutRadii::roomy();

    // Basic geometry
    let center_x = 300.0_f64;
    let center_y = 250.0_f64;

    // Core node radii
    let master_r = radii.radius_for("root");
    let defs_r   = radii.radius_for("category");
    let abbr_r   = radii.radius_for("category");

    // distance from master center to category centers
    // must be >= sum of radii + padding + optional extra offset
    let cat_radius = (master_r + defs_r + radii.padding + radii.category_extra_offset)
        .max(master_r + abbr_r + radii.padding + radii.category_extra_offset);

    let defs_center = (center_x - cat_radius, center_y);
    let abbr_center = (center_x + cat_radius, center_y);

    // Ring radii (either provided explicitly or computed safely)
    let alpha_radius = radii.alpha_ring_radius.unwrap_or_else(|| radii.alpha_ring_radius_default());
    let leaf_radius  = radii.leaf_ring_radius.unwrap_or_else(|| radii.leaf_ring_radius_default());
    let abbr_ring_radius = radii.abbr_ring_radius.unwrap_or_else(|| radii.abbr_ring_radius_default());

    // Split by kind (term_nodes isn't used below in your snippet; keep only what you use)
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

    // ----- alpha nodes around Definitions -----
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

        vedges.push(VisualEdge {
            from: defs_id.clone(),
            to: node.id.clone(),
            kind: "category".into(),
        });
    }

    // ----- Abbr nodes around Abbreviations -----
    let abbr_limit = abbr_nodes.len().min(40);
    for (i, node) in abbr_nodes.iter().take(abbr_limit).enumerate() {
        let angle = (i as f64) / (abbr_limit as f64).max(1.0) * std::f64::consts::TAU;
        let x = abbr_center.0 + abbr_ring_radius * angle.cos();
        let y = abbr_center.1 + abbr_ring_radius * angle.sin();

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

