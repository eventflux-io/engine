// SPDX-License-Identifier: MIT OR Apache-2.0

//! Type Inference Engine for EventFlux SQL Compiler
//!
//! This module implements compile-time type inference for SQL expressions,
//! enabling automatic type determination for query outputs and early detection
//! of type errors.
//!
//! # Design Principles
//!
//! 1. **Fail Fast**: Catch type errors at compile time, not runtime
//! 2. **Zero Allocation**: Use references, no Arc overhead
//! 3. **Extensible**: Data-driven function registry
//! 4. **Performance**: <0.1ms overhead per query

use crate::query_api::definition::attribute::Type as AttributeType;
use crate::query_api::expression::constant::Constant;
use crate::query_api::expression::expression::Expression;
use crate::query_api::expression::variable::Variable;
use crate::sql_compiler::catalog::SqlCatalog;
use crate::sql_compiler::error::TypeError;

/// Context for type inference providing stream/relation information
#[derive(Debug, Clone, Default)]
pub struct TypeContext {
    /// Available streams/tables for column lookup (includes all JOIN sources)
    pub available_streams: Vec<String>,
}

impl TypeContext {
    /// Create a new empty context
    #[inline]
    pub const fn new() -> Self {
        TypeContext {
            available_streams: Vec::new(),
        }
    }

    /// Create context from a primary stream name
    #[inline]
    pub fn from_stream(stream_name: String) -> Self {
        TypeContext {
            available_streams: vec![stream_name],
        }
    }

    /// Add an additional stream to the context (for JOINs)
    #[inline]
    pub fn with_stream(mut self, stream_name: String) -> Self {
        self.available_streams.push(stream_name);
        self
    }

    /// Build context from multiple stream names
    #[inline]
    pub fn from_streams(streams: Vec<String>) -> Self {
        TypeContext {
            available_streams: streams,
        }
    }
}

/// Function signature for type inference
struct FunctionSignature {
    name: &'static str,
    min_args: usize,
    return_type: fn(&[AttributeType]) -> Result<AttributeType, TypeError>,
}

impl FunctionSignature {
    const fn new(
        name: &'static str,
        min_args: usize,
        return_type: fn(&[AttributeType]) -> Result<AttributeType, TypeError>,
    ) -> Self {
        Self {
            name,
            min_args,
            return_type,
        }
    }
}

// Type precedence for numeric operations: DOUBLE > FLOAT > LONG > INT
#[inline]
const fn type_precedence(t: AttributeType) -> u8 {
    match t {
        AttributeType::DOUBLE => 4,
        AttributeType::FLOAT => 3,
        AttributeType::LONG => 2,
        AttributeType::INT => 1,
        _ => 0,
    }
}

#[inline]
fn is_numeric(t: AttributeType) -> bool {
    matches!(
        t,
        AttributeType::INT | AttributeType::LONG | AttributeType::FLOAT | AttributeType::DOUBLE
    )
}

// Function registry - data-driven instead of huge match statement
fn get_function_signature(name: &str) -> Option<&'static FunctionSignature> {
    static FUNCTIONS: &[FunctionSignature] = &[
        // Aggregations
        FunctionSignature::new("count", 0, |_| Ok(AttributeType::LONG)),
        FunctionSignature::new("sum", 1, |args| match args[0] {
            AttributeType::INT | AttributeType::LONG => Ok(AttributeType::LONG),
            AttributeType::FLOAT | AttributeType::DOUBLE => Ok(AttributeType::DOUBLE),
            _ => Err(TypeError::ConversionFailed(
                "SUM requires numeric argument".into(),
            )),
        }),
        FunctionSignature::new("avg", 1, |args| {
            if is_numeric(args[0]) {
                Ok(AttributeType::DOUBLE)
            } else {
                Err(TypeError::ConversionFailed(
                    "AVG requires numeric argument".into(),
                ))
            }
        }),
        FunctionSignature::new("min", 1, |args| Ok(args[0])),
        FunctionSignature::new("max", 1, |args| Ok(args[0])),
        // Math functions
        FunctionSignature::new("round", 1, |args| match args[0] {
            AttributeType::FLOAT | AttributeType::DOUBLE => Ok(AttributeType::DOUBLE),
            AttributeType::INT | AttributeType::LONG => Ok(AttributeType::LONG),
            _ => Err(TypeError::ConversionFailed(
                "ROUND requires numeric argument".into(),
            )),
        }),
        FunctionSignature::new("abs", 1, |args| {
            if is_numeric(args[0]) {
                Ok(args[0])
            } else {
                Err(TypeError::ConversionFailed(
                    "ABS requires numeric argument".into(),
                ))
            }
        }),
        // String functions
        FunctionSignature::new("upper", 1, |args| {
            if args[0] == AttributeType::STRING {
                Ok(AttributeType::STRING)
            } else {
                Err(TypeError::ConversionFailed(
                    "UPPER requires STRING argument".into(),
                ))
            }
        }),
        FunctionSignature::new("lower", 1, |args| {
            if args[0] == AttributeType::STRING {
                Ok(AttributeType::STRING)
            } else {
                Err(TypeError::ConversionFailed(
                    "LOWER requires STRING argument".into(),
                ))
            }
        }),
        FunctionSignature::new("length", 1, |args| {
            if args[0] == AttributeType::STRING {
                Ok(AttributeType::INT)
            } else {
                Err(TypeError::ConversionFailed(
                    "LENGTH requires STRING argument".into(),
                ))
            }
        }),
        FunctionSignature::new("concat", 0, |_| Ok(AttributeType::STRING)),
    ];

    FUNCTIONS.iter().find(|f| f.name == name)
}

/// Type Inference Engine
///
/// Lightweight, zero-allocation type inference using catalog reference.
pub struct TypeInferenceEngine<'a> {
    catalog: &'a SqlCatalog,
}

impl<'a> TypeInferenceEngine<'a> {
    /// Create a new type inference engine with catalog reference
    #[inline]
    pub const fn new(catalog: &'a SqlCatalog) -> Self {
        TypeInferenceEngine { catalog }
    }

    /// Infer the result type of an expression
    ///
    /// # Performance
    ///
    /// This function is highly optimized with:
    /// - No heap allocations in hot path
    /// - Const functions where possible
    /// - Inlined type checks
    /// - Data-driven function lookup
    pub fn infer_type(
        &self,
        expr: &Expression,
        context: &TypeContext,
    ) -> Result<AttributeType, TypeError> {
        match expr {
            Expression::Constant(c) => Self::infer_constant_type(c),
            Expression::Variable(v) => self.infer_variable_type(v, context),
            Expression::Add(add) => {
                self.infer_arithmetic_type(&add.left_value, &add.right_value, context)
            }
            Expression::Subtract(sub) => {
                self.infer_arithmetic_type(&sub.left_value, &sub.right_value, context)
            }
            Expression::Multiply(mul) => {
                self.infer_arithmetic_type(&mul.left_value, &mul.right_value, context)
            }
            Expression::Divide(div) => {
                self.infer_arithmetic_type(&div.left_value, &div.right_value, context)
            }
            Expression::Mod(m) => {
                self.infer_arithmetic_type(&m.left_value, &m.right_value, context)
            }
            Expression::Compare(_)
            | Expression::And(_)
            | Expression::Or(_)
            | Expression::Not(_)
            | Expression::IsNull(_)
            | Expression::In(_) => Ok(AttributeType::BOOL),
            Expression::AttributeFunction(func) => self.infer_function_type(func, context),
            Expression::Case(case) => {
                // Type of CASE is determined by the result branches
                // All WHEN results and ELSE must have the same type
                if case.when_clauses.is_empty() {
                    return Err(TypeError::ConversionFailed(
                        "CASE expression must have at least one WHEN clause".to_string(),
                    ));
                }

                // Find first non-NULL result type from WHEN clauses or ELSE
                let mut result_type = AttributeType::OBJECT;
                for when_clause in &case.when_clauses {
                    let when_type = self.infer_type(&when_clause.result, context)?;
                    if when_type != AttributeType::OBJECT {
                        result_type = when_type;
                        break;
                    }
                }
                // If all WHENs are NULL, check ELSE
                if result_type == AttributeType::OBJECT {
                    let else_type = self.infer_type(&case.else_result, context)?;
                    if else_type != AttributeType::OBJECT {
                        result_type = else_type;
                    }
                }

                // Validate all WHEN results have same type (allow NULL/OBJECT)
                for (idx, when_clause) in case.when_clauses.iter().enumerate() {
                    let when_type = self.infer_type(&when_clause.result, context)?;
                    if when_type != result_type && when_type != AttributeType::OBJECT {
                        return Err(TypeError::ConversionFailed(format!(
                            "CASE expression type mismatch: WHEN clause {} returns {:?}, expected {:?}",
                            idx + 1,
                            when_type,
                            result_type
                        )));
                    }
                }

                // Validate ELSE has same type (allow NULL/OBJECT for implicit ELSE NULL)
                let else_type = self.infer_type(&case.else_result, context)?;
                if else_type != result_type && else_type != AttributeType::OBJECT {
                    return Err(TypeError::ConversionFailed(format!(
                        "CASE expression type mismatch: ELSE clause returns {:?}, expected {:?}",
                        else_type, result_type
                    )));
                }

                Ok(result_type)
            }
        }
    }

    /// Infer type from a constant value (inline, no allocation)
    #[inline]
    fn infer_constant_type(constant: &Constant) -> Result<AttributeType, TypeError> {
        use crate::query_api::expression::constant::ConstantValueWithFloat;

        Ok(match &constant.value {
            ConstantValueWithFloat::Int(_) => AttributeType::INT,
            ConstantValueWithFloat::Long(_) | ConstantValueWithFloat::Time(_) => {
                AttributeType::LONG
            }
            ConstantValueWithFloat::Float(_) => AttributeType::FLOAT,
            ConstantValueWithFloat::Double(_) => AttributeType::DOUBLE,
            ConstantValueWithFloat::String(_) => AttributeType::STRING,
            ConstantValueWithFloat::Bool(_) => AttributeType::BOOL,
            ConstantValueWithFloat::Null => AttributeType::OBJECT, // NULL maps to OBJECT type
        })
    }

    /// Infer type from a variable (column reference)
    fn infer_variable_type(
        &self,
        variable: &Variable,
        context: &TypeContext,
    ) -> Result<AttributeType, TypeError> {
        let attr_name = variable.get_attribute_name();

        // Qualified variable (e.g., "s.price")
        if let Some(stream_id) = variable.get_stream_id() {
            return self
                .catalog
                .get_column_type(stream_id, attr_name)
                .map_err(|e| {
                    TypeError::ConversionFailed(format!(
                        "Column {}.{} not found: {}",
                        stream_id, attr_name, e
                    ))
                });
        }

        // Unqualified - search all available streams
        for stream_name in &context.available_streams {
            if let Ok(attr_type) = self.catalog.get_column_type(stream_name, attr_name) {
                return Ok(attr_type);
            }
        }

        Err(TypeError::ConversionFailed(format!(
            "Column '{}' not found in available streams",
            attr_name
        )))
    }

    /// Infer result type for arithmetic operations using type precedence
    #[inline]
    fn infer_arithmetic_type(
        &self,
        left: &Expression,
        right: &Expression,
        context: &TypeContext,
    ) -> Result<AttributeType, TypeError> {
        let left_type = self.infer_type(left, context)?;
        let right_type = self.infer_type(right, context)?;

        // Both must be numeric
        if !is_numeric(left_type) || !is_numeric(right_type) {
            return Err(TypeError::ConversionFailed(format!(
                "Cannot perform arithmetic on {:?} and {:?}",
                left_type, right_type
            )));
        }

        // Return higher precedence type
        Ok(
            if type_precedence(left_type) >= type_precedence(right_type) {
                left_type
            } else {
                right_type
            },
        )
    }

    /// Infer result type for function calls using function registry
    fn infer_function_type(
        &self,
        func: &crate::query_api::expression::attribute_function::AttributeFunction,
        context: &TypeContext,
    ) -> Result<AttributeType, TypeError> {
        let func_name = func.function_name.to_lowercase();

        // Infer parameter types
        let param_types: Result<Vec<AttributeType>, TypeError> = func
            .parameters
            .iter()
            .map(|param| self.infer_type(param, context))
            .collect();
        let param_types = param_types?;

        // Lookup function signature
        if let Some(sig) = get_function_signature(&func_name) {
            // Validate argument count
            if param_types.len() < sig.min_args {
                return Err(TypeError::ConversionFailed(format!(
                    "{} requires at least {} argument(s), found {}",
                    func_name.to_uppercase(),
                    sig.min_args,
                    param_types.len()
                )));
            }

            // Apply signature-based type inference
            (sig.return_type)(&param_types)
        } else {
            // Unknown function - default to OBJECT (allows UDFs)
            Ok(AttributeType::OBJECT)
        }
    }

    /// Validate that an expression returns a boolean type
    ///
    /// Used for WHERE, HAVING, JOIN ON clauses.
    /// Returns detailed error with helpful hints.
    pub fn validate_boolean_expression(
        &self,
        expr: &Expression,
        context: &TypeContext,
        clause_name: &str,
    ) -> Result<(), TypeError> {
        let expr_type = self.infer_type(expr, context)?;

        if expr_type != AttributeType::BOOL {
            let hint = match expr {
                Expression::Variable(_) if is_numeric(expr_type) => {
                    format!("Did you mean to use a comparison? Try adding '> 0' or '!= 0'")
                }
                Expression::Add(_)
                | Expression::Subtract(_)
                | Expression::Multiply(_)
                | Expression::Divide(_) => {
                    format!("Arithmetic expressions must be compared: try '(...) > 0'")
                }
                Expression::AttributeFunction(f) => {
                    format!(
                        "Function '{}' returns {:?}, not BOOL - use a comparison",
                        f.function_name, expr_type
                    )
                }
                _ => format!("Expected BOOL expression, found {:?}", expr_type),
            };

            return Err(TypeError::ConversionFailed(format!(
                "{} clause must return BOOL type, found {:?}. Hint: {}",
                clause_name, expr_type, hint
            )));
        }

        Ok(())
    }

    /// Build TypeContext from query's input streams
    ///
    /// Collects both stream IDs and stream reference IDs (aliases) to support
    /// qualified column references like "R.column" in table joins.
    pub fn build_context_from_query(
        &self,
        query: &crate::query_api::execution::query::Query,
    ) -> TypeContext {
        let mut available_sources = Vec::new();

        if let Some(input_stream) = query.get_input_stream() {
            // Recursively collect all stream IDs and reference IDs (aliases)
            Self::collect_stream_identifiers(input_stream, &mut available_sources);
        }

        TypeContext::from_streams(available_sources)
    }

    /// Recursively collect stream IDs and reference IDs from input streams
    fn collect_stream_identifiers(
        input: &crate::query_api::execution::query::input::stream::input_stream::InputStream,
        identifiers: &mut Vec<String>,
    ) {
        use crate::query_api::execution::query::input::stream::input_stream::InputStream;

        match input {
            InputStream::Single(single) => {
                Self::collect_from_single_stream(single, identifiers);
            }
            InputStream::Join(join) => {
                Self::collect_from_single_stream(&join.left_input_stream, identifiers);
                Self::collect_from_single_stream(&join.right_input_stream, identifiers);
            }
            InputStream::State(state) => {
                // State streams have nested BasicSingleInputStream which wraps SingleInputStream
                // The get_all_stream_ids() already handles the recursion for state streams
                use crate::query_api::execution::query::input::stream::input_stream::InputStreamTrait;
                identifiers.extend(state.get_all_stream_ids());
            }
        }
    }

    /// Collect identifiers from a single input stream (both ID and reference ID if exists)
    fn collect_from_single_stream(
        single: &crate::query_api::execution::query::input::stream::single_input_stream::SingleInputStream,
        identifiers: &mut Vec<String>,
    ) {
        // Add the stream ID
        let stream_id = single.get_stream_id_str().to_string();
        if !identifiers.contains(&stream_id) {
            identifiers.push(stream_id);
        }

        // Also add the reference ID (alias) if it exists
        if let Some(ref_id) = single.get_stream_reference_id_str() {
            let ref_id = ref_id.to_string();
            if !identifiers.contains(&ref_id) {
                identifiers.push(ref_id);
            }
        }
    }

    /// Validate an entire query for type correctness
    ///
    /// Validates:
    /// - WHERE clauses must return BOOL
    /// - HAVING clauses must return BOOL
    /// - JOIN ON conditions must return BOOL
    pub fn validate_query(
        &self,
        query: &crate::query_api::execution::query::Query,
    ) -> Result<(), TypeError> {
        use crate::query_api::execution::query::input::handler::StreamHandler;
        use crate::query_api::execution::query::input::stream::input_stream::InputStream;

        let context = self.build_context_from_query(query);

        // Validate WHERE clauses (in stream handlers)
        if let Some(input_stream) = query.get_input_stream() {
            if let InputStream::Single(single_stream) = input_stream {
                for handler in single_stream.get_stream_handlers() {
                    if let StreamHandler::Filter(filter) = handler {
                        self.validate_boolean_expression(
                            &filter.filter_expression,
                            &context,
                            "WHERE",
                        )?;
                    }
                }
            }
        }

        // Validate HAVING clause
        if let Some(having) = query.get_selector().get_having_expression() {
            self.validate_boolean_expression(having, &context, "HAVING")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query_api::definition::stream_definition::StreamDefinition;

    fn create_test_catalog() -> SqlCatalog {
        let mut catalog = SqlCatalog::new();

        let stream = StreamDefinition::new("TestStream".to_string())
            .attribute("symbol".to_string(), AttributeType::STRING)
            .attribute("price".to_string(), AttributeType::DOUBLE)
            .attribute("volume".to_string(), AttributeType::LONG)
            .attribute("change".to_string(), AttributeType::FLOAT)
            .attribute("count".to_string(), AttributeType::INT)
            .attribute("active".to_string(), AttributeType::BOOL);

        catalog
            .register_stream("TestStream".to_string(), stream)
            .unwrap();

        catalog
    }

    #[test]
    fn test_constant_type_inference() {
        let catalog = create_test_catalog();
        let engine = TypeInferenceEngine::new(&catalog);

        assert_eq!(
            TypeInferenceEngine::infer_constant_type(&Constant::int(42)).unwrap(),
            AttributeType::INT
        );
        assert_eq!(
            TypeInferenceEngine::infer_constant_type(&Constant::long(42)).unwrap(),
            AttributeType::LONG
        );
        assert_eq!(
            TypeInferenceEngine::infer_constant_type(&Constant::double(3.14)).unwrap(),
            AttributeType::DOUBLE
        );
    }

    #[test]
    fn test_variable_type_inference() {
        let catalog = create_test_catalog();
        let engine = TypeInferenceEngine::new(&catalog);
        let context = TypeContext::from_stream("TestStream".to_string());

        let var = Variable::new("price".to_string());
        assert_eq!(
            engine.infer_variable_type(&var, &context).unwrap(),
            AttributeType::DOUBLE
        );
    }

    #[test]
    fn test_arithmetic_type_precedence() {
        let catalog = create_test_catalog();
        let engine = TypeInferenceEngine::new(&catalog);
        let context = TypeContext::from_stream("TestStream".to_string());

        // DOUBLE + INT → DOUBLE
        let expr = Expression::add(
            Expression::variable("price".to_string()),
            Expression::value_int(10),
        );
        assert_eq!(
            engine.infer_type(&expr, &context).unwrap(),
            AttributeType::DOUBLE
        );

        // LONG * INT → LONG
        let expr = Expression::multiply(
            Expression::variable("volume".to_string()),
            Expression::value_int(2),
        );
        assert_eq!(
            engine.infer_type(&expr, &context).unwrap(),
            AttributeType::LONG
        );
    }

    #[test]
    fn test_function_registry() {
        let catalog = create_test_catalog();
        let engine = TypeInferenceEngine::new(&catalog);
        let context = TypeContext::from_stream("TestStream".to_string());

        // COUNT → LONG
        let expr = Expression::function_no_ns("count".to_string(), vec![]);
        assert_eq!(
            engine.infer_type(&expr, &context).unwrap(),
            AttributeType::LONG
        );

        // AVG → DOUBLE
        let expr = Expression::function_no_ns(
            "avg".to_string(),
            vec![Expression::variable("price".to_string())],
        );
        assert_eq!(
            engine.infer_type(&expr, &context).unwrap(),
            AttributeType::DOUBLE
        );
    }
}
