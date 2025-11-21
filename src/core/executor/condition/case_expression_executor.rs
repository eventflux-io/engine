// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::core::config::eventflux_app_context::EventFluxAppContext;
use crate::core::event::complex_event::ComplexEvent;
use crate::core::event::value::AttributeValue;
use crate::core::executor::expression_executor::ExpressionExecutor;
use crate::query_api::definition::attribute::Type as ApiAttributeType;
use crate::query_api::expression::Case;
use std::fmt;
use std::sync::Arc;

/// Executor for CASE expressions (both Searched and Simple CASE)
///
/// Searched CASE: CASE WHEN condition THEN result ... ELSE default END
/// Simple CASE: CASE operand WHEN value THEN result ... ELSE default END
///
/// SQL-92 Semantics:
/// - Type validation at construction time (all results must be same type)
/// - NULL handling: NULL != NULL (CASE NULL WHEN NULL returns ELSE)
/// - Short-circuit evaluation: stops at first matching WHEN
/// - ELSE is mandatory (injected as NULL by parser if missing)
pub struct CaseExpressionExecutor {
    /// Optional operand for Simple CASE (CASE expr WHEN val...)
    /// None for Searched CASE (CASE WHEN condition...)
    operand_executor: Option<Arc<dyn ExpressionExecutor>>,

    /// WHEN clauses: (condition/value, result)
    when_executors: Vec<WhenClauseExecutor>,

    /// ELSE result (always present - parser injects NULL if missing)
    else_executor: Arc<dyn ExpressionExecutor>,

    /// Result type (validated at construction)
    result_type: ApiAttributeType,
}

struct WhenClauseExecutor {
    condition_executor: Arc<dyn ExpressionExecutor>,
    result_executor: Arc<dyn ExpressionExecutor>,
}

impl CaseExpressionExecutor {
    pub fn new(
        _case: &Case,
        operand_executor: Option<Arc<dyn ExpressionExecutor>>,
        when_executors: Vec<(Arc<dyn ExpressionExecutor>, Arc<dyn ExpressionExecutor>)>,
        else_executor: Arc<dyn ExpressionExecutor>,
    ) -> Result<Self, String> {
        // Validate that we have at least one WHEN clause
        if when_executors.is_empty() {
            return Err("CASE expression must have at least one WHEN clause".to_string());
        }

        // Find first non-NULL result type from WHEN clauses or ELSE
        let mut result_type = ApiAttributeType::OBJECT;
        for (_, result_exec) in &when_executors {
            let when_type = result_exec.get_return_type();
            if when_type != ApiAttributeType::OBJECT {
                result_type = when_type;
                break;
            }
        }
        // If all WHENs are NULL, check ELSE
        if result_type == ApiAttributeType::OBJECT {
            let else_type = else_executor.get_return_type();
            if else_type != ApiAttributeType::OBJECT {
                result_type = else_type;
            }
        }

        // Validate all WHEN results have same type (allow NULL/OBJECT)
        for (idx, (_, result_exec)) in when_executors.iter().enumerate() {
            let when_type = result_exec.get_return_type();
            if when_type != result_type && when_type != ApiAttributeType::OBJECT {
                return Err(format!(
                    "CASE expression type mismatch: WHEN clause {} returns {:?}, expected {:?}",
                    idx + 1,
                    when_type,
                    result_type
                ));
            }
        }

        // Validate ELSE result has same type (allow OBJECT for implicit ELSE NULL)
        let else_type = else_executor.get_return_type();
        if else_type != result_type && else_type != ApiAttributeType::OBJECT {
            return Err(format!(
                "CASE expression type mismatch: ELSE clause returns {:?}, expected {:?}",
                else_type, result_type
            ));
        }

        // For Simple CASE, validate that operand and WHEN values have compatible types
        if let Some(ref operand_exec) = operand_executor {
            let operand_type = operand_exec.get_return_type();
            for (idx, (when_exec, _)) in when_executors.iter().enumerate() {
                let when_type = when_exec.get_return_type();
                if !Self::are_comparable_types(&operand_type, &when_type) {
                    return Err(format!(
                        "CASE expression Simple CASE type mismatch: operand type {:?} not comparable with WHEN clause {} type {:?}",
                        operand_type,
                        idx + 1,
                        when_type
                    ));
                }
            }
        }

        Ok(Self {
            operand_executor,
            when_executors: when_executors
                .into_iter()
                .map(|(cond, res)| WhenClauseExecutor {
                    condition_executor: cond,
                    result_executor: res,
                })
                .collect(),
            else_executor,
            result_type,
        })
    }

    /// Check if two types are comparable for Simple CASE
    fn are_comparable_types(type1: &ApiAttributeType, type2: &ApiAttributeType) -> bool {
        // Same types are always comparable
        if type1 == type2 {
            return true;
        }

        // Numeric types are comparable with each other
        let numeric_types = [
            ApiAttributeType::INT,
            ApiAttributeType::LONG,
            ApiAttributeType::FLOAT,
            ApiAttributeType::DOUBLE,
        ];

        let is_type1_numeric = numeric_types.contains(type1);
        let is_type2_numeric = numeric_types.contains(type2);

        is_type1_numeric && is_type2_numeric
    }

    /// Compare two AttributeValues for equality (SQL semantics: NULL != NULL)
    /// Supports cross-type numeric comparison (Int/Long/Float/Double)
    fn sql_equals(left: &AttributeValue, right: &AttributeValue) -> bool {
        // NULL is not equal to anything, including NULL
        if matches!(left, AttributeValue::Null) || matches!(right, AttributeValue::Null) {
            return false;
        }

        // Cross-type numeric comparison
        match (left, right) {
            // Same type comparisons
            (AttributeValue::Int(a), AttributeValue::Int(b)) => a == b,
            (AttributeValue::Long(a), AttributeValue::Long(b)) => a == b,
            (AttributeValue::Float(a), AttributeValue::Float(b)) => a == b,
            (AttributeValue::Double(a), AttributeValue::Double(b)) => a == b,

            // Cross-type integer comparisons (Int <-> Long)
            (AttributeValue::Int(a), AttributeValue::Long(b)) => (*a as i64) == *b,
            (AttributeValue::Long(a), AttributeValue::Int(b)) => *a == (*b as i64),

            // Cross-type float comparisons (Float <-> Double)
            (AttributeValue::Float(a), AttributeValue::Double(b)) => (*a as f64) == *b,
            (AttributeValue::Double(a), AttributeValue::Float(b)) => *a == (*b as f64),

            // Integer to float comparisons
            (AttributeValue::Int(a), AttributeValue::Float(b)) => (*a as f32) == *b,
            (AttributeValue::Float(a), AttributeValue::Int(b)) => *a == (*b as f32),
            (AttributeValue::Int(a), AttributeValue::Double(b)) => (*a as f64) == *b,
            (AttributeValue::Double(a), AttributeValue::Int(b)) => *a == (*b as f64),
            (AttributeValue::Long(a), AttributeValue::Float(b)) => (*a as f32) == *b,
            (AttributeValue::Float(a), AttributeValue::Long(b)) => *a == (*b as f32),
            (AttributeValue::Long(a), AttributeValue::Double(b)) => (*a as f64) == *b,
            (AttributeValue::Double(a), AttributeValue::Long(b)) => *a == (*b as f64),

            // Non-numeric types use standard equality
            _ => left == right,
        }
    }
}

impl ExpressionExecutor for CaseExpressionExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let event = event?;

        if let Some(ref operand_exec) = self.operand_executor {
            // Simple CASE: CASE operand WHEN value1 THEN result1...
            let operand_value = operand_exec.execute(Some(event))?;

            // Short-circuit evaluation: return first matching WHEN
            for when_clause in &self.when_executors {
                let when_value = when_clause.condition_executor.execute(Some(event))?;

                // SQL semantics: NULL != NULL
                if Self::sql_equals(&operand_value, &when_value) {
                    return when_clause.result_executor.execute(Some(event));
                }
            }

            // No WHEN matched, return ELSE
            self.else_executor.execute(Some(event))
        } else {
            // Searched CASE: CASE WHEN condition1 THEN result1...
            for when_clause in &self.when_executors {
                let condition_value = when_clause.condition_executor.execute(Some(event))?;

                // WHEN condition must evaluate to boolean
                match condition_value {
                    AttributeValue::Bool(true) => {
                        return when_clause.result_executor.execute(Some(event));
                    }
                    AttributeValue::Bool(false) | AttributeValue::Null => {
                        // Continue to next WHEN
                        continue;
                    }
                    _ => {
                        // Invalid condition type - return None
                        return None;
                    }
                }
            }

            // No WHEN matched, return ELSE
            self.else_executor.execute(Some(event))
        }
    }

    fn get_return_type(&self) -> ApiAttributeType {
        self.result_type
    }

    fn clone_executor(
        &self,
        eventflux_app_context: &Arc<EventFluxAppContext>,
    ) -> Box<dyn ExpressionExecutor> {
        Box::new(Self {
            operand_executor: self
                .operand_executor
                .as_ref()
                .map(|exec| Arc::from(exec.clone_executor(eventflux_app_context))),
            when_executors: self
                .when_executors
                .iter()
                .map(|when_clause| WhenClauseExecutor {
                    condition_executor: Arc::from(
                        when_clause
                            .condition_executor
                            .clone_executor(eventflux_app_context),
                    ),
                    result_executor: Arc::from(
                        when_clause
                            .result_executor
                            .clone_executor(eventflux_app_context),
                    ),
                })
                .collect(),
            else_executor: Arc::from(self.else_executor.clone_executor(eventflux_app_context)),
            result_type: self.result_type,
        })
    }
}

impl fmt::Debug for CaseExpressionExecutor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CaseExpressionExecutor")
            .field("is_simple_case", &self.operand_executor.is_some())
            .field("num_when_clauses", &self.when_executors.len())
            .field("result_type", &self.result_type)
            .finish()
    }
}

// Unit tests are covered by comprehensive integration tests in tests/app_runner_case_expression.rs
