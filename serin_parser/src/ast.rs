use serde::Serialize;

/// Top-level SQL statement enumeration.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Statement {
    /// `SELECT` statement.
    Select(Select),
    /// `INSERT` statement.
    Insert,
    /// `UPDATE` statement.
    Update,
    /// `DELETE` statement.
    Delete,
    /// Cypher-like graph query.
    GraphQuery(CypherQuery),
}

/// Very small `SELECT` representation (placeholder for full AST).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Select {
    /// Projection items, `*` or expressions.
    pub projection: Vec<SelectItem>,
}

/// Projection item.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum SelectItem {
    /// Asterisk.
    Star,
    /// Numeric literal.
    Number(i64),
}

/// Simple Cypher-like graph query AST.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CypherQuery {
    /// Queried variable name, e.g., `n` in MATCH (n)
    pub variable: String,
} 