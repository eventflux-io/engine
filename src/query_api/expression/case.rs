// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::query_api::eventflux_element::EventFluxElement;
use crate::query_api::expression::Expression;

/// CASE expression for conditional logic
/// Supports both Searched CASE (CASE WHEN condition THEN result...)
/// and Simple CASE (CASE operand WHEN value THEN result...)
#[derive(Clone, Debug, PartialEq)]
pub struct Case {
    /// Optional operand for simple CASE (CASE expr WHEN val...)
    pub operand: Option<Box<Expression>>,
    /// WHEN clauses: (condition/value, result)
    pub when_clauses: Vec<WhenClause>,
    /// ELSE result (required - ensures type consistency)
    pub else_result: Box<Expression>,
    /// Source location for error reporting
    pub eventflux_element: EventFluxElement,
}

/// A single WHEN clause in a CASE expression
#[derive(Clone, Debug, PartialEq)]
pub struct WhenClause {
    pub condition: Box<Expression>,
    pub result: Box<Expression>,
}

impl Case {
    pub fn new(
        operand: Option<Box<Expression>>,
        when_clauses: Vec<WhenClause>,
        else_result: Box<Expression>,
    ) -> Self {
        Self {
            operand,
            when_clauses,
            else_result,
            eventflux_element: EventFluxElement::default(),
        }
    }
}

impl WhenClause {
    pub fn new(condition: Box<Expression>, result: Box<Expression>) -> Self {
        Self { condition, result }
    }
}
