//! B+Tree implementation (minimal, in-memory, order 4).
#![deny(missing_docs)]
use serde::{Deserialize, Serialize};

const ORDER: usize = 4; // max keys per node

/// Key type.
pub type Key = i32;
/// Value type.
pub type Value = i64;

/// B+Tree node.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum Node {
    /// Internal node with keys and child pointers.
    Internal {
        keys: Vec<Key>,
        children: Vec<Box<Node>>, // len = keys.len()+1
    },
    /// Leaf node with key-value pairs and next pointer (chain).
    Leaf {
        keys: Vec<Key>,
        values: Vec<Value>,
        next: Option<Box<Node>>, // link to right sibling
    },
}

impl Node {
    fn is_leaf(&self) -> bool {
        matches!(self, Node::Leaf { .. })
    }
}

/// B+Tree structure.
#[derive(Debug)]
pub struct BPlusTree {
    root: Box<Node>,
}

impl Default for BPlusTree {
    fn default() -> Self {
        Self {
            root: Box::new(Node::Leaf {
                keys: Vec::new(),
                values: Vec::new(),
                next: None,
            }),
        }
    }
}

impl BPlusTree {
    /// Search for a key, returning Option<Value>.
    pub fn search(&self, key: Key) -> Option<Value> {
        let mut node = &self.root;
        loop {
            match &**node {
                Node::Internal { keys, children } => {
                    let idx = keys.iter().position(|&k| key < k).unwrap_or(keys.len());
                    node = &children[idx];
                }
                Node::Leaf { keys, values, .. } => {
                    return keys
                        .iter()
                        .position(|&k| k == key)
                        .map(|i| values[i]);
                }
            }
        }
    }

    /// Insert key-value pair.
    pub fn insert(&mut self, key: Key, value: Value) {
        let (split_key, split_node) = Self::insert_inner(&mut self.root, key, value);
        if let Some((k, node_box)) = split_node.map(|n| (split_key.unwrap(), n)) {
            // create new root
            let old_root = std::mem::replace(&mut self.root, Box::new(Node::Leaf { keys: vec![], values: vec![], next: None }));
            self.root = Box::new(Node::Internal {
                keys: vec![k],
                children: vec![old_root, node_box],
            });
        }
    }

    fn insert_inner(node: &mut Box<Node>, key: Key, value: Value) -> (Option<Key>, Option<Box<Node>>) {
        match node.as_mut() {
            Node::Leaf { keys, values, .. } => {
                let idx = keys.iter().position(|&k| k >= key).unwrap_or(keys.len());
                keys.insert(idx, key);
                values.insert(idx, value);
                if keys.len() > ORDER {
                    // split
                    let split_point = keys.len() / 2;
                    let right_keys = keys.split_off(split_point);
                    let right_values = values.split_off(split_point);
                    let split_key = right_keys[0];
                    let new_leaf = Box::new(Node::Leaf {
                        keys: right_keys,
                        values: right_values,
                        next: None,
                    });
                    return (Some(split_key), Some(new_leaf));
                }
                (None, None)
            }
            Node::Internal { keys, children } => {
                let idx = keys.iter().position(|&k| key < k).unwrap_or(keys.len());
                let (split_key, split_child) = Self::insert_inner(&mut children[idx], key, value);
                if let Some(child) = split_child {
                    keys.insert(idx, split_key.unwrap());
                    children.insert(idx + 1, child);
                    if keys.len() > ORDER {
                        let split_point = keys.len() / 2;
                        let right_keys = keys.split_off(split_point + 1);
                        let promo_key = keys.pop().unwrap();
                        let right_children = children.split_off(split_point + 1);
                        let new_internal = Box::new(Node::Internal {
                            keys: right_keys,
                            children: right_children,
                        });
                        return (Some(promo_key), Some(new_internal));
                    }
                }
                (None, None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_search() {
        let mut tree = BPlusTree::default();
        for i in 0..1000 {
            tree.insert(i, i as i64 * 10);
        }
        for i in 0..1000 {
            assert_eq!(tree.search(i), Some(i as i64 * 10));
        }
    }
} 