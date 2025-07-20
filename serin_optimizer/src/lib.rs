//! SerinDB logical plan generator (MVP).
#![deny(missing_docs)]

use serde::{Deserialize, Serialize};
use serin_parser::{SelectItem, Statement};

/// Logical plan node enumeration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LogicalPlan {
    /// Scan over a table.
    Scan { table: String },
    /// Selection predicate.
    Filter { predicate: String, input: Box<LogicalPlan> },
    /// Projection.
    Project { items: Vec<SelectItem>, input: Box<LogicalPlan> },
}

/// Generate a logical plan from parsed AST.
pub fn plan(stmt: &Statement) -> Option<LogicalPlan> {
    match stmt {
        Statement::Select(sel) => {
            // For MVP, assume scan of dummy table "dual".
            let scan = LogicalPlan::Scan {
                table: "dual".to_string(),
            };
            Some(LogicalPlan::Project {
                items: sel.projection.clone(),
                input: Box::new(scan),
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serin_parser::{parse, SelectItem};

    #[test]
    fn project_plan() {
        let ast = parse("SELECT 1;").unwrap();
        let plan = plan(&ast).unwrap();
        if let LogicalPlan::Project { items, .. } = plan {
            assert_eq!(items, vec![SelectItem::Number(1)]);
        } else {
            panic!("expected project plan");
        }
    }
} 