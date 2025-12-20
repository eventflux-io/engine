// SPDX-License-Identifier: MIT OR Apache-2.0

//! CAST Expression
//!
//! Represents a type conversion expression: CAST(expr AS type)

use super::expression::Expression;
use crate::query_api::definition::attribute::Type as AttributeType;
use crate::query_api::eventflux_element::EventFluxElement;

/// CAST expression for type conversion
///
/// SQL syntax: `CAST(expression AS target_type)`
///
/// Supports conversions between:
/// - String to numeric types (INT, LONG, FLOAT, DOUBLE)
/// - Numeric types to String
/// - Numeric type widening (INT -> LONG, FLOAT -> DOUBLE)
/// - Numeric type narrowing (LONG -> INT, DOUBLE -> FLOAT)
#[derive(Clone, Debug, PartialEq)]
pub struct Cast {
    /// The expression to convert
    pub expression: Box<Expression>,
    /// The target type to convert to
    pub target_type: AttributeType,
    /// EventFlux element metadata
    pub eventflux_element: EventFluxElement,
}

impl Cast {
    /// Create a new Cast expression
    pub fn new(expression: Expression, target_type: AttributeType) -> Self {
        Self {
            expression: Box::new(expression),
            target_type,
            eventflux_element: EventFluxElement::default(),
        }
    }
}
