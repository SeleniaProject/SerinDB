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

/// Physical plan operators.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PhysicalPlan {
    /// Sequential table scan.
    SeqScan { table: String, cost: f64 },
    /// Projection executed by materialization.
    Projection { child: Box<PhysicalPlan>, cost: f64 },
}

/// Estimate cost for a logical plan and choose physical operators (very naive).
pub fn physical_from(logical: &LogicalPlan) -> PhysicalPlan {
    match logical {
        LogicalPlan::Scan { table } => PhysicalPlan::SeqScan {
            table: table.clone(),
            cost: 100.0, // constant for MVP
        },
        LogicalPlan::Project { items: _, input } => {
            let child = physical_from(input);
            let child_cost = cost(&child);
            PhysicalPlan::Projection {
                child: Box::new(child),
                cost: child_cost + 10.0,
            }
        }
        _ => todo!(),
    }
}

/// Extract cost from physical plan recursively.
pub fn cost(plan: &PhysicalPlan) -> f64 {
    match plan {
        PhysicalPlan::SeqScan { cost, .. } => *cost,
        PhysicalPlan::Projection { cost, .. } => *cost,
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

#[cfg(test)]
mod phys_tests {
    use super::*;
    use serin_parser::parse;

    #[test]
    fn select_physical_plan() {
        let ast = parse("SELECT 1;").unwrap();
        let logical = plan(&ast).unwrap();
        let phys = physical_from(&logical);
        assert!(cost(&phys) > 0.0);
    }
} 