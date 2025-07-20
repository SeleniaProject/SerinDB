use std::collections::{HashMap, HashSet, VecDeque};
use serde::{Serialize, Deserialize};
use anyhow::{Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct NodeId(pub u64);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EdgeId(pub u64);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub labels: Vec<String>,
    pub props: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: EdgeId,
    pub from: NodeId,
    pub to: NodeId,
    pub label: String,
    pub props: HashMap<String, String>,
}

#[derive(Default)]
pub struct Graph {
    pub nodes: HashMap<NodeId, Node>,
    pub edges: HashMap<EdgeId, Edge>,
    adjacency: HashMap<NodeId, Vec<EdgeId>>, // out edges
}

impl Graph {
    pub fn add_node(&mut self, node: Node) { self.nodes.insert(node.id.clone(), node); }
    pub fn add_edge(&mut self, edge: Edge) {
        self.adjacency.entry(edge.from.clone()).or_default().push(edge.id.clone());
        self.edges.insert(edge.id.clone(), edge);
    }

    /// Breadth-First traversal returning visited node ids.
    pub fn bfs(&self, start: &NodeId, max_depth: usize) -> Vec<NodeId> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        visited.insert(start.clone());
        queue.push_back((start.clone(), 0usize));
        let mut order = vec![start.clone()];
        while let Some((nid, depth)) = queue.pop_front() {
            if depth >= max_depth { continue; }
            if let Some(edges) = self.adjacency.get(&nid) {
                for eid in edges {
                    if let Some(edge) = self.edges.get(eid) {
                        if visited.insert(edge.to.clone()) {
                            order.push(edge.to.clone());
                            queue.push_back((edge.to.clone(), depth + 1));
                        }
                    }
                }
            }
        }
        order
    }
}

/// Very minimal Cypher-like parser: accepts `MATCH (a)-[]->(b)` returns vector of patterns.
pub fn parse_cypher(query: &str) -> Result<&'static str> {
    if query.trim_start().to_ascii_uppercase().starts_with("MATCH") {
        Ok("MATCH")
    } else if query.trim_start().to_ascii_uppercase().starts_with("CREATE") {
        Ok("CREATE")
    } else {
        anyhow::bail!("unsupported cypher")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bfs_traverse() {
        let mut g = Graph::default();
        g.add_node(Node { id: NodeId(1), labels: vec!["Person".into()], props: HashMap::new() });
        g.add_node(Node { id: NodeId(2), labels: vec!["Person".into()], props: HashMap::new() });
        g.add_edge(Edge { id: EdgeId(1), from: NodeId(1), to: NodeId(2), label: "KNOWS".into(), props: HashMap::new() });
        let vis = g.bfs(&NodeId(1), 2);
        assert_eq!(vis, vec![NodeId(1), NodeId(2)]);
    }
} 