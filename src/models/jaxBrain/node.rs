use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub data: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Edge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub relation: String,
    pub data: Value,
}

// ===== NEW: layout types =====

#[derive(Debug, Clone, PartialEq)]
pub struct VisualGraph {
    pub nodes: Vec<VisualNode>,
    pub edges: Vec<VisualEdge>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualNode {
    pub id: String,
    pub label: String,
    pub kind: String,  // "root", "category", "term", "abbreviation", etc
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualEdge {
    pub from: String,
    pub to: String,
    pub kind: String,  // "hierarchy", "category", "related"
}
