use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Directory,
    File,
    Section,
    Task,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Contains,
    References,
    Sequence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Node {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: NodeKind,
    pub source: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Edge {
    pub source: String,
    pub target: String,
    #[serde(rename = "type")]
    pub kind: EdgeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl Graph {
    pub fn add(&mut self, other: Graph) {
        self.nodes.extend(other.nodes);
        self.edges.extend(other.edges);
    }

    pub fn sort(&mut self) {
        self.nodes.sort_by(|a, b| a.id.cmp(&b.id));
        self.edges
            .sort_by(|a, b| (&a.source, &a.target, &a.kind).cmp(&(&b.source, &b.target, &b.kind)));
    }
}
