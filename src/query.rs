use crate::graph::{Edge, EdgeKind, Graph, Node, NodeKind};
use std::collections::{HashSet, VecDeque};

pub enum Traversal {
    None,
    Roots,
    Children(String),
    Descendants(String),
    Ancestors(String),
    References(String),
    ReferencedBy(String),
    Next(String),
}

pub enum FilterOp {
    Eq,
    Contains,
}

pub struct Filter {
    pub field: String,
    pub op: FilterOp,
    pub value: String,
}

impl Filter {
    pub fn parse(s: &str) -> Option<Filter> {
        if let Some((field, value)) = s.split_once('~') {
            Some(Filter { field: field.into(), op: FilterOp::Contains, value: value.into() })
        } else if let Some((field, value)) = s.split_once('=') {
            Some(Filter { field: field.into(), op: FilterOp::Eq, value: value.into() })
        } else {
            Option::None
        }
    }

    pub fn matches(&self, node: &Node) -> bool {
        let Some(val) = node_field_value(node, &self.field) else {
            return false;
        };
        match self.op {
            FilterOp::Eq => val == self.value,
            FilterOp::Contains => val.to_ascii_lowercase().contains(&self.value.to_ascii_lowercase()),
        }
    }
}

fn kind_str(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::Directory => "directory",
        NodeKind::File => "file",
        NodeKind::Section => "section",
        NodeKind::Task => "task",
    }
}

fn node_field_value(node: &Node, field: &str) -> Option<String> {
    match field {
        "type" => Some(kind_str(&node.kind).to_string()),
        "source" => Some(node.source.clone()),
        "label" => Some(node.label.clone()),
        "id" => Some(node.id.clone()),
        _ => {
            let obj = node.metadata.as_ref()?.as_object()?;
            obj.get(field)?.as_str().map(|s| s.to_string())
        }
    }
}

pub fn execute(graph: &Graph, traversal: &Traversal, filters: &[Filter]) -> Graph {
    // Step 1: traversal → candidate node IDs
    let candidate_ids: HashSet<&str> = match traversal {
        Traversal::None => graph.nodes.iter().map(|n| n.id.as_str()).collect(),
        Traversal::Roots => {
            let targets: HashSet<&str> = graph.edges.iter().map(|e| e.target.as_str()).collect();
            graph.nodes.iter().map(|n| n.id.as_str()).filter(|id| !targets.contains(id)).collect()
        }
        Traversal::Children(id) => {
            graph.edges.iter()
                .filter(|e| e.source == *id && e.kind == EdgeKind::Contains)
                .map(|e| e.target.as_str()).collect()
        }
        Traversal::Descendants(id) => {
            let mut result = HashSet::new();
            let mut queue = VecDeque::new();
            queue.push_back(id.as_str());
            while let Some(cur) = queue.pop_front() {
                for e in &graph.edges {
                    if e.source == cur && e.kind == EdgeKind::Contains && result.insert(e.target.as_str()) {
                        queue.push_back(e.target.as_str());
                    }
                }
            }
            result
        }
        Traversal::Ancestors(id) => {
            let mut result = HashSet::new();
            let mut queue = VecDeque::new();
            queue.push_back(id.as_str());
            while let Some(cur) = queue.pop_front() {
                for e in &graph.edges {
                    if e.target == cur && e.kind == EdgeKind::Contains && result.insert(e.source.as_str()) {
                        queue.push_back(e.source.as_str());
                    }
                }
            }
            result
        }
        Traversal::References(id) => {
            graph.edges.iter()
                .filter(|e| e.source == *id && e.kind == EdgeKind::References)
                .map(|e| e.target.as_str())
                .collect()
        }
        Traversal::ReferencedBy(id) => {
            graph.edges.iter()
                .filter(|e| e.target == *id && e.kind == EdgeKind::References)
                .map(|e| e.source.as_str())
                .collect()
        }
        Traversal::Next(id) => {
            graph.edges.iter()
                .filter(|e| e.source == *id && e.kind == EdgeKind::Sequence)
                .map(|e| e.target.as_str())
                .collect()
        }
    };

    // Step 2: apply filters
    let nodes: Vec<Node> = graph
        .nodes
        .iter()
        .filter(|n| candidate_ids.contains(n.id.as_str()))
        .filter(|n| filters.iter().all(|f| f.matches(n)))
        .cloned()
        .collect();

    let surviving: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();

    // Step 3: prune edges
    let edges: Vec<Edge> = graph
        .edges
        .iter()
        .filter(|e| surviving.contains(e.source.as_str()) && surviving.contains(e.target.as_str()))
        .cloned()
        .collect();

    Graph { nodes, edges }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::EdgeKind;

    fn test_graph() -> Graph {
        let nodes = vec![
            Node { id: "root".into(), kind: NodeKind::Section, source: "markdown".into(), label: "Root".into(), metadata: None },
            Node { id: "child1".into(), kind: NodeKind::Task, source: "markdown".into(), label: "Scan feature".into(),
                metadata: Some(serde_json::json!({"status": "done"})) },
            Node { id: "child2".into(), kind: NodeKind::Task, source: "markdown".into(), label: "Query feature".into(),
                metadata: Some(serde_json::json!({"status": "todo"})) },
            Node { id: "grandchild".into(), kind: NodeKind::Task, source: "markdown".into(), label: "Sub task".into(),
                metadata: Some(serde_json::json!({"status": "todo"})) },
            Node { id: "file1".into(), kind: NodeKind::File, source: "filesystem".into(), label: "main.rs".into(), metadata: None },
        ];
        let edges = vec![
            Edge { source: "root".into(), target: "child1".into(), kind: EdgeKind::Contains },
            Edge { source: "root".into(), target: "child2".into(), kind: EdgeKind::Contains },
            Edge { source: "child2".into(), target: "grandchild".into(), kind: EdgeKind::Contains },
        ];
        Graph { nodes, edges }
    }

    #[test]
    fn filter_parse_eq() {
        let f = Filter::parse("type=task").unwrap();
        assert!(matches!(f.op, FilterOp::Eq));
        assert_eq!(f.field, "type");
        assert_eq!(f.value, "task");
    }

    #[test]
    fn filter_parse_contains() {
        let f = Filter::parse("label~scan").unwrap();
        assert!(matches!(f.op, FilterOp::Contains));
        assert_eq!(f.field, "label");
        assert_eq!(f.value, "scan");
    }

    #[test]
    fn filter_by_type() {
        let g = test_graph();
        let filters = vec![Filter::parse("type=task").unwrap()];
        let result = execute(&g, &Traversal::None, &filters);
        assert_eq!(result.nodes.len(), 3);
        assert!(result.nodes.iter().all(|n| matches!(n.kind, NodeKind::Task)));
    }

    #[test]
    fn filter_by_metadata() {
        let g = test_graph();
        let filters = vec![Filter::parse("status=todo").unwrap()];
        let result = execute(&g, &Traversal::None, &filters);
        assert_eq!(result.nodes.len(), 2);
    }

    #[test]
    fn filter_label_contains() {
        let g = test_graph();
        let filters = vec![Filter::parse("label~scan").unwrap()];
        let result = execute(&g, &Traversal::None, &filters);
        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.nodes[0].id, "child1");
    }

    #[test]
    fn traversal_roots() {
        let g = test_graph();
        let result = execute(&g, &Traversal::Roots, &[]);
        let ids: HashSet<&str> = result.nodes.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains("root"));
        assert!(ids.contains("file1"));
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn traversal_children() {
        let g = test_graph();
        let result = execute(&g, &Traversal::Children("root".into()), &[]);
        let ids: Vec<&str> = result.nodes.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains(&"child1"));
        assert!(ids.contains(&"child2"));
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn traversal_descendants() {
        let g = test_graph();
        let result = execute(&g, &Traversal::Descendants("root".into()), &[]);
        assert_eq!(result.nodes.len(), 3); // child1, child2, grandchild
    }

    #[test]
    fn traversal_ancestors() {
        let g = test_graph();
        let result = execute(&g, &Traversal::Ancestors("grandchild".into()), &[]);
        let ids: HashSet<&str> = result.nodes.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains("child2"));
        assert!(ids.contains("root"));
    }

    #[test]
    fn combined_traversal_and_filter() {
        let g = test_graph();
        let filters = vec![Filter::parse("status=todo").unwrap()];
        let result = execute(&g, &Traversal::Descendants("root".into()), &filters);
        assert_eq!(result.nodes.len(), 2); // child2 + grandchild
    }

    #[test]
    fn edges_pruned() {
        let g = test_graph();
        let filters = vec![Filter::parse("type=task").unwrap()];
        let result = execute(&g, &Traversal::None, &filters);
        // Only edge between tasks survives: child2 -> grandchild
        assert_eq!(result.edges.len(), 1);
        assert_eq!(result.edges[0].source, "child2");
    }

    fn ref_graph() -> Graph {
        let nodes = vec![
            Node { id: "a".into(), kind: NodeKind::Section, source: "markdown".into(), label: "A".into(), metadata: None },
            Node { id: "b".into(), kind: NodeKind::File, source: "filesystem".into(), label: "B".into(), metadata: None },
            Node { id: "c".into(), kind: NodeKind::Section, source: "markdown".into(), label: "C".into(), metadata: None },
        ];
        let edges = vec![
            Edge { source: "a".into(), target: "b".into(), kind: EdgeKind::References },
            Edge { source: "a".into(), target: "c".into(), kind: EdgeKind::References },
            Edge { source: "c".into(), target: "b".into(), kind: EdgeKind::References },
            Edge { source: "a".into(), target: "c".into(), kind: EdgeKind::Contains },
        ];
        Graph { nodes, edges }
    }

    #[test]
    fn traversal_references() {
        let g = ref_graph();
        let result = execute(&g, &Traversal::References("a".into()), &[]);
        let ids: HashSet<&str> = result.nodes.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains("b"));
        assert!(ids.contains("c"));
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn traversal_referenced_by() {
        let g = ref_graph();
        let result = execute(&g, &Traversal::ReferencedBy("b".into()), &[]);
        let ids: HashSet<&str> = result.nodes.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains("a"));
        assert!(ids.contains("c"));
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn references_with_filter() {
        let g = ref_graph();
        let filters = vec![Filter::parse("type=file").unwrap()];
        let result = execute(&g, &Traversal::References("a".into()), &filters);
        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.nodes[0].id, "b");
    }
}
