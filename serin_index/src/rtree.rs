//! R-Tree implementation with STR (Sort-Tile-Recursive) bulk loading split algorithm.
//! This version supports 2-D axis-aligned rectangles and basic intersection queries.
//! It is intentionally lightweight (no disk persistence yet) but designed to be
//! integrated into GiST-like framework later.

use serde::{Deserialize, Serialize};

/// Maximum number of entries per node (M). Chosen as 16 to balance depth/fan-out.
const MAX_ENTRIES: usize = 16;
/// Minimum number of entries per node (m). Common practice m ≈ M/2.
const MIN_ENTRIES: usize = MAX_ENTRIES / 2;

/// A 2-D axis-aligned bounding rectangle.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Rect {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl Rect {
    /// Create new rectangle.
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self { min_x, min_y, max_x, max_y }
    }

    /// Returns rectangle that encloses both `self` and `other`.
    pub fn union(&self, other: &Rect) -> Rect {
        Rect {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }

    /// Area of rectangle.
    fn area(&self) -> f64 { (self.max_x - self.min_x) * (self.max_y - self.min_y) }

    /// Compute enlargement needed to include `other`.
    fn enlargement(&self, other: &Rect) -> f64 { self.union(other).area() - self.area() }

    /// Check intersection with another rectangle.
    pub fn intersects(&self, other: &Rect) -> bool {
        !(self.max_x < other.min_x || self.min_x > other.max_x || self.max_y < other.min_y || self.min_y > other.max_y)
    }
}

/// Entry in a leaf node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeafEntry<T> {
    pub rect: Rect,
    pub value: T,
}

/// Node either leaf or internal.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum Node<T> {
    Leaf { bbox: Rect, entries: Vec<LeafEntry<T>> },
    Internal { bbox: Rect, children: Vec<Box<Node<T>>> },
}

impl<T: Clone> Node<T> {
    fn bbox(&self) -> Rect {
        match self {
            Node::Leaf { bbox, .. } => *bbox,
            Node::Internal { bbox, .. } => *bbox,
        }
    }

    /// Recompute bounding box from children or entries.
    fn refresh_bbox(&mut self) {
        match self {
            Node::Leaf { bbox, entries } => {
                let mut b = entries[0].rect;
                for e in entries.iter().skip(1) { b = b.union(&e.rect); }
                *bbox = b;
            }
            Node::Internal { bbox, children } => {
                let mut b = children[0].bbox();
                for c in children.iter().skip(1) { b = b.union(&c.bbox()); }
                *bbox = b;
            }
        }
    }

    /// Choose leaf for insertion (linear pick, could use R* heuristics later).
    fn choose_leaf(&mut self, rect: &Rect) -> &mut Node<T> {
        match self {
            Node::Leaf { .. } => self,
            Node::Internal { children, .. } => {
                // Pick child requiring least enlargement, tie-break smaller area.
                let mut best_idx = 0;
                let mut best_enl = children[0].bbox().enlargement(rect);
                let mut best_area = children[0].bbox().area();
                for (i, child) in children.iter().enumerate().skip(1) {
                    let enl = child.bbox().enlargement(rect);
                    let area = child.bbox().area();
                    if enl < best_enl || (enl == best_enl && area < best_area) {
                        best_idx = i;
                        best_enl = enl;
                        best_area = area;
                    }
                }
                children[best_idx].choose_leaf(rect)
            }
        }
    }

    /// Split node using STR algorithm; returns sibling node.
    fn split_str(node_vec: &mut Vec<Box<Node<T>>>) -> Vec<Box<Node<T>>> {
        // Flatten children/entries to rectangles with node pointers for sorting.
        let mut items: Vec<(Rect, Box<Node<T>>)> = node_vec
            .drain(..)
            .map(|n| {
                let rect = n.bbox();
                (rect, n)
            })
            .collect();
        // STR bulk-loading procedure for 2-D:
        // 1. Sort by min_x, tile into S stripes
        // 2. Within each stripe, sort by min_y and pack groups of M entries.
        let n = items.len();
        let m = MAX_ENTRIES;
        let s = ((n as f64).sqrt()).ceil() as usize; // number of stripes ≈ sqrt(n)
        // Step1 sort by X
        items.sort_by(|a, b| a.0.min_x.partial_cmp(&b.0.min_x).unwrap());
        let stripe_size = ((n + s - 1) / s).max(m); // entries per stripe
        let mut new_nodes: Vec<Box<Node<T>>> = Vec::new();
        for stripe in items.chunks(stripe_size) {
            let mut stripe_vec: Vec<(Rect, Box<Node<T>>)> = stripe.to_vec();
            // sort by Y
            stripe_vec.sort_by(|a, b| a.0.min_y.partial_cmp(&b.0.min_y).unwrap());
            // pack
            for chunk in stripe_vec.chunks(m) {
                let mut children: Vec<Box<Node<T>>> = chunk.iter().map(|(_, n)| n.clone()).collect();
                let mut bbox = children[0].bbox();
                for c in children.iter().skip(1) { bbox = bbox.union(&c.bbox()); }
                new_nodes.push(Box::new(Node::Internal { bbox, children }));
            }
        }
        new_nodes
    }

    /// Insert child into internal node, splitting as necessary.
    fn add_child(&mut self, child: Box<Node<T>>) -> Option<Box<Node<T>>> {
        match self {
            Node::Leaf { .. } => unreachable!(),
            Node::Internal { children, bbox } => {
                children.push(child);
                *bbox = bbox.union(&children.last().unwrap().bbox());
                if children.len() > MAX_ENTRIES {
                    // Need to split children into multiple internal nodes.
                    let mut nodes = std::mem::take(children);
                    let new_nodes = Self::split_str(&mut nodes);
                    // After split, first element becomes current node's children, the rest are siblings.
                    *children = if let Node::Internal { children, .. } = *new_nodes[0].clone() { children } else { unreachable!() };
                    self.refresh_bbox();
                    // Return sibling nodes (excluding self).
                    return Some(Box::new(Node::Internal {
                        bbox: new_nodes[1].bbox(),
                        children: if let Node::Internal { children, .. } = *new_nodes[1].clone() { children } else { unreachable!() },
                    }));
                }
                None
            }
        }
    }

    /// Insert into tree, handle splits recursively.
    fn insert(&mut self, entry: LeafEntry<T>) -> Option<Box<Node<T>>> {
        match self {
            Node::Leaf { entries, bbox } => {
                entries.push(entry);
                *bbox = bbox.union(&entries.last().unwrap().rect);
                if entries.len() > MAX_ENTRIES {
                    // Split leaf.
                    // Convert LeafEntry to Node::Leaf boxed for STR split reuse.
                    let mut leaf_boxes: Vec<Box<Node<T>>> = entries
                        .drain(..)
                        .map(|e| {
                            let b = e.rect;
                            Box::new(Node::Leaf { bbox: b, entries: vec![e.clone()] })
                        })
                        .collect();
                    let new_nodes = Self::split_str(&mut leaf_boxes);
                    // Rebuild this node from first cluster.
                    if let Node::Internal { children, .. } = &*new_nodes[0] {
                        let mut new_entries: Vec<LeafEntry<T>> = Vec::new();
                        for child in children {
                            if let Node::Leaf { entries, .. } = &**child {
                                new_entries.extend_from_slice(entries);
                            }
                        }
                        *entries = new_entries;
                        self.refresh_bbox();
                    }
                    // sibling node constructed from next cluster
                    if new_nodes.len() > 1 {
                        return Some(new_nodes[1].clone());
                    }
                }
                None
            }
            Node::Internal { children, .. } => {
                // choose leaf>> insert
                let rect = entry.rect;
                let target = self.choose_leaf(&rect);
                if let Some(split_node) = target.insert(entry) {
                    // Add newly split sibling to current node.
                    return self.add_child(split_node);
                }
                self.refresh_bbox();
                None
            }
        }
    }

    /// Search intersection with query rectangle.
    fn search(&self, query: &Rect, results: &mut Vec<T>) {
        match self {
            Node::Leaf { entries, .. } => {
                for e in entries {
                    if e.rect.intersects(query) {
                        results.push(e.value.clone());
                    }
                }
            }
            Node::Internal { children, .. } => {
                for child in children {
                    if child.bbox().intersects(query) {
                        child.search(query, results);
                    }
                }
            }
        }
    }
}

/// R-Tree structure.
#[derive(Debug)]
pub struct RTree<T> {
    root: Box<Node<T>>,
}

impl<T: Clone> Default for RTree<T> {
    fn default() -> Self {
        let bbox = Rect::new(0.0, 0.0, 0.0, 0.0);
        Self { root: Box::new(Node::Leaf { bbox, entries: Vec::new() }) }
    }
}

impl<T: Clone> RTree<T> {
    /// Insert a rectangle with associated value.
    pub fn insert(&mut self, rect: Rect, value: T) {
        let entry = LeafEntry { rect, value };
        if let Some(sibling) = self.root.insert(entry) {
            // Root split – create new root with two children.
            let mut new_children = vec![std::mem::replace(&mut self.root, sibling)];
            new_children.push(self.root.clone());
            let mut bbox = new_children[0].bbox();
            bbox = bbox.union(&new_children[1].bbox());
            self.root = Box::new(Node::Internal { bbox, children: new_children });
        }
    }

    /// Search for all entries whose rectangles intersect `query`.
    pub fn search(&self, query: &Rect) -> Vec<T> {
        let mut results = Vec::new();
        self.root.search(query, &mut results);
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_query() {
        let mut tree = RTree::default();
        for i in 0..100 {
            let r = Rect::new(i as f64, i as f64, (i + 1) as f64, (i + 1) as f64);
            tree.insert(r, i);
        }
        let q = Rect::new(10.5, 10.5, 20.5, 20.5);
        let res = tree.search(&q);
        assert_eq!(res.len(), 10);
        assert!(res.contains(&15));
    }
} 