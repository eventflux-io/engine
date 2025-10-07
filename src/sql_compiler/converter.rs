// SPDX-License-Identifier: MIT OR Apache-2.0

//! SQL to Query Converter
//!
//! Converts SQL statements to EventFlux query_api::Query structures.

use sqlparser::ast::{
    BinaryOperator, Expr as SqlExpr, JoinConstraint, JoinOperator, OrderByExpr,
    Select as SqlSelect, SetExpr, Statement, TableFactor, UnaryOperator,
};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

use crate::query_api::execution::query::input::stream::input_stream::InputStream;
use crate::query_api::execution::query::input::stream::single_input_stream::SingleInputStream;
use crate::query_api::execution::query::output::output_stream::{
    InsertIntoStreamAction, OutputStream, OutputStreamAction,
};
use crate::query_api::execution::query::selection::selector::Selector;
use crate::query_api::execution::query::Query;
use crate::query_api::expression::variable::Variable;
use crate::query_api::expression::CompareOperator;
use crate::query_api::expression::Expression;

use super::catalog::SqlCatalog;
use super::error::ConverterError;
use super::expansion::SelectExpander;
use super::preprocessor::{SqlPreprocessor, WindowSpec};

/// SQL to Query Converter
pub struct SqlConverter;

impl SqlConverter {
    /// Convert SQL query to Query
    pub fn convert(sql: &str, catalog: &SqlCatalog) -> Result<Query, ConverterError> {
        // Step 1: Preprocess to extract WINDOW clause
        let preprocessed = SqlPreprocessor::preprocess(sql)
            .map_err(|e| ConverterError::ConversionFailed(e.to_string()))?;

        // Step 2: Parse SQL
        let statements = Parser::parse_sql(&GenericDialect, &preprocessed.standard_sql)
            .map_err(|e| ConverterError::ConversionFailed(format!("SQL parse error: {}", e)))?;

        if statements.is_empty() {
            return Err(ConverterError::ConversionFailed(
                "No SQL statements found".to_string(),
            ));
        }

        // Step 3: Convert SELECT or INSERT INTO statement to Query
        match &statements[0] {
            Statement::Query(query) => {
                // Plain SELECT query - output to default "OutputStream"
                Self::convert_query(query, catalog, preprocessed.window_clause.as_ref(), None)
            }
            Statement::Insert {
                table_name, source, ..
            } => {
                // INSERT INTO TargetStream SELECT ... - extract target stream name
                let target_stream = table_name.to_string();
                Self::convert_query(
                    source,
                    catalog,
                    preprocessed.window_clause.as_ref(),
                    Some(target_stream),
                )
            }
            _ => Err(ConverterError::UnsupportedFeature(
                "Only SELECT and INSERT INTO queries supported in M1".to_string(),
            )),
        }
    }

    /// Convert sqlparser Query to EventFlux Query
    fn convert_query(
        sql_query: &sqlparser::ast::Query,
        catalog: &SqlCatalog,
        window_spec: Option<&super::preprocessor::WindowClauseText>,
        output_stream_name: Option<String>,
    ) -> Result<Query, ConverterError> {
        match sql_query.body.as_ref() {
            SetExpr::Select(select) => Self::convert_select(
                select,
                catalog,
                window_spec,
                &sql_query.order_by,
                sql_query.limit.as_ref(),
                sql_query.offset.as_ref(),
                output_stream_name,
            ),
            _ => Err(ConverterError::UnsupportedFeature(
                "Only simple SELECT supported in M1".to_string(),
            )),
        }
    }

    /// Convert SELECT statement to Query
    fn convert_select(
        select: &SqlSelect,
        catalog: &SqlCatalog,
        window_spec: Option<&super::preprocessor::WindowClauseText>,
        order_by: &[OrderByExpr],
        limit: Option<&SqlExpr>,
        offset: Option<&sqlparser::ast::Offset>,
        output_stream_name: Option<String>,
    ) -> Result<Query, ConverterError> {
        // Check if this is a JOIN query
        let has_join = !select.from.is_empty() && !select.from[0].joins.is_empty();

        let input_stream = if has_join {
            // Handle JOIN
            Self::convert_join_from_clause(&select.from, &select.selection, catalog)?
        } else {
            // Handle single stream
            let stream_name = Self::extract_from_stream(&select.from)?;

            // Validate stream exists
            catalog
                .get_stream(&stream_name)
                .map_err(|_| ConverterError::SchemaNotFound(stream_name.clone()))?;

            // Create InputStream
            let mut single_stream = SingleInputStream::new_basic(
                stream_name.clone(),
                false,      // is_inner_stream
                false,      // is_fault_stream
                None,       // stream_handler_id
                Vec::new(), // pre_window_handlers
            );

            // Add WINDOW if present
            if let Some(window) = window_spec {
                single_stream = Self::add_window(single_stream, window)?;
            }

            // Add WHERE filter (BEFORE aggregation)
            if let Some(where_expr) = &select.selection {
                let filter_expr = Self::convert_expression(where_expr, catalog)?;
                single_stream = single_stream.filter(filter_expr);
            }

            InputStream::Single(single_stream)
        };

        // Create Selector from SELECT clause
        // For JOIN queries, we don't have a single stream name - use empty string as fallback
        let stream_name_for_selector = if has_join {
            String::new() // JOIN queries use qualified names (table.column)
        } else {
            Self::extract_from_stream(&select.from)?
        };

        let mut selector = SelectExpander::expand_select_items(
            &select.projection,
            &stream_name_for_selector,
            catalog,
        )
        .map_err(|e| ConverterError::ConversionFailed(e.to_string()))?;

        // Add GROUP BY if present
        if let sqlparser::ast::GroupByExpr::Expressions(group_exprs) = &select.group_by {
            for expr in group_exprs {
                if let SqlExpr::Identifier(ident) = expr {
                    selector = selector.group_by(Variable::new(ident.value.clone()));
                } else {
                    return Err(ConverterError::UnsupportedFeature(
                        "Complex GROUP BY expressions not supported in M1".to_string(),
                    ));
                }
            }
        }

        // Add HAVING (AFTER aggregation)
        if let Some(having) = &select.having {
            let having_expr = Self::convert_expression(having, catalog)?;
            selector = selector.having(having_expr);
        }

        // Add ORDER BY
        for order_expr in order_by {
            // Extract variable from order_expr.expr
            let variable = match &order_expr.expr {
                SqlExpr::Identifier(ident) => Variable::new(ident.value.clone()),
                SqlExpr::CompoundIdentifier(idents) => {
                    if idents.len() == 1 {
                        Variable::new(idents[0].value.clone())
                    } else {
                        return Err(ConverterError::UnsupportedFeature(
                            "Qualified column names in ORDER BY not supported".to_string(),
                        ));
                    }
                }
                _ => {
                    return Err(ConverterError::UnsupportedFeature(
                        "Complex expressions in ORDER BY not supported in M1".to_string(),
                    ))
                }
            };

            // Determine order (ASC/DESC)
            let order = if let Some(asc) = order_expr.asc {
                if asc {
                    crate::query_api::execution::query::selection::order_by_attribute::Order::Asc
                } else {
                    crate::query_api::execution::query::selection::order_by_attribute::Order::Desc
                }
            } else {
                // Default to ASC if not specified
                crate::query_api::execution::query::selection::order_by_attribute::Order::Asc
            };

            selector = selector.order_by_with_order(variable, order);
        }

        // Add LIMIT
        if let Some(limit_expr) = limit {
            let limit_const = Self::convert_to_constant(limit_expr)?;
            selector = selector
                .limit(limit_const)
                .map_err(|e| ConverterError::ConversionFailed(format!("LIMIT error: {}", e)))?;
        }

        // Add OFFSET
        if let Some(offset_obj) = offset {
            let offset_const = Self::convert_to_constant(&offset_obj.value)?;
            selector = selector
                .offset(offset_const)
                .map_err(|e| ConverterError::ConversionFailed(format!("OFFSET error: {}", e)))?;
        }

        // Create output stream (use provided name or default to "OutputStream")
        let target_stream_name = output_stream_name.unwrap_or_else(|| "OutputStream".to_string());
        let output_action = InsertIntoStreamAction {
            target_id: target_stream_name,
            is_inner_stream: false,
            is_fault_stream: false,
        };
        let output_stream = OutputStream::new(OutputStreamAction::InsertInto(output_action), None);

        // Build Query
        Ok(Query::query()
            .from(input_stream)
            .select(selector)
            .out_stream(output_stream))
    }

    /// Extract stream name from FROM clause
    fn extract_from_stream(
        from: &[sqlparser::ast::TableWithJoins],
    ) -> Result<String, ConverterError> {
        if from.is_empty() {
            return Err(ConverterError::ConversionFailed(
                "No FROM clause found".to_string(),
            ));
        }

        match &from[0].relation {
            TableFactor::Table { name, .. } => name
                .0
                .last()
                .map(|ident| ident.value.clone())
                .ok_or_else(|| {
                    ConverterError::ConversionFailed("No table name in FROM".to_string())
                }),
            _ => Err(ConverterError::UnsupportedFeature(
                "Complex FROM clauses not supported in M1".to_string(),
            )),
        }
    }

    /// Convert JOIN from clause to JoinInputStream
    fn convert_join_from_clause(
        from: &[sqlparser::ast::TableWithJoins],
        where_clause: &Option<SqlExpr>,
        catalog: &SqlCatalog,
    ) -> Result<InputStream, ConverterError> {
        use crate::query_api::execution::query::input::stream::join_input_stream::{
            EventTrigger, JoinInputStream, Type as JoinType,
        };

        if from.is_empty() || from[0].joins.is_empty() {
            return Err(ConverterError::ConversionFailed(
                "No JOIN found in FROM clause".to_string(),
            ));
        }

        // Extract left stream
        let left_stream_name = match &from[0].relation {
            TableFactor::Table { name, alias, .. } => {
                let stream_name =
                    name.0
                        .last()
                        .map(|ident| ident.value.clone())
                        .ok_or_else(|| {
                            ConverterError::ConversionFailed("No left table name".to_string())
                        })?;

                // Validate stream exists
                catalog
                    .get_stream(&stream_name)
                    .map_err(|_| ConverterError::SchemaNotFound(stream_name.clone()))?;

                let mut left_stream = SingleInputStream::new_basic(
                    stream_name.clone(),
                    false,
                    false,
                    None,
                    Vec::new(),
                );

                // Add alias if present
                if let Some(table_alias) = alias {
                    left_stream = left_stream.as_ref(table_alias.name.value.clone());
                }

                left_stream
            }
            _ => {
                return Err(ConverterError::UnsupportedFeature(
                    "Complex left table in JOIN".to_string(),
                ))
            }
        };

        // Get first JOIN (only support single JOIN for M1)
        let join = &from[0].joins[0];

        // Extract right stream
        let right_stream_name = match &join.relation {
            TableFactor::Table { name, alias, .. } => {
                let stream_name =
                    name.0
                        .last()
                        .map(|ident| ident.value.clone())
                        .ok_or_else(|| {
                            ConverterError::ConversionFailed("No right table name".to_string())
                        })?;

                // Validate stream exists
                catalog
                    .get_stream(&stream_name)
                    .map_err(|_| ConverterError::SchemaNotFound(stream_name.clone()))?;

                let mut right_stream = SingleInputStream::new_basic(
                    stream_name.clone(),
                    false,
                    false,
                    None,
                    Vec::new(),
                );

                // Add alias if present
                if let Some(table_alias) = alias {
                    right_stream = right_stream.as_ref(table_alias.name.value.clone());
                }

                right_stream
            }
            _ => {
                return Err(ConverterError::UnsupportedFeature(
                    "Complex right table in JOIN".to_string(),
                ))
            }
        };

        // Extract join type
        let join_type = match &join.join_operator {
            JoinOperator::Inner(_) => JoinType::InnerJoin,
            JoinOperator::LeftOuter(_) => JoinType::LeftOuterJoin,
            JoinOperator::RightOuter(_) => JoinType::RightOuterJoin,
            JoinOperator::FullOuter(_) => JoinType::FullOuterJoin,
            _ => JoinType::Join, // Default JOIN
        };

        // Extract ON condition
        let on_condition = match &join.join_operator {
            JoinOperator::Inner(JoinConstraint::On(expr))
            | JoinOperator::LeftOuter(JoinConstraint::On(expr))
            | JoinOperator::RightOuter(JoinConstraint::On(expr))
            | JoinOperator::FullOuter(JoinConstraint::On(expr)) => {
                Some(Self::convert_expression(expr, catalog)?)
            }
            _ => None,
        };

        // Create JoinInputStream
        let join_stream = JoinInputStream::new(
            left_stream_name,
            join_type,
            right_stream_name,
            on_condition,
            EventTrigger::All, // Default trigger
            None,              // No WITHIN clause for M1
            None,              // No PER clause for M1
        );

        Ok(InputStream::Join(Box::new(join_stream)))
    }

    /// Add window to SingleInputStream
    fn add_window(
        stream: SingleInputStream,
        window: &super::preprocessor::WindowClauseText,
    ) -> Result<SingleInputStream, ConverterError> {
        use super::preprocessor::TimeUnit;

        match &window.spec {
            WindowSpec::Tumbling { value, unit } => {
                let time_expr = Self::create_time_expression(*value, unit)?;
                Ok(stream.window(None, "timeBatch".to_string(), vec![time_expr]))
            }
            WindowSpec::Sliding {
                window_value,
                window_unit,
                slide_value: _,
                slide_unit: _,
            } => {
                let time_expr = Self::create_time_expression(*window_value, window_unit)?;
                Ok(stream.window(None, "time".to_string(), vec![time_expr]))
            }
            WindowSpec::Length { size } => Ok(stream.window(
                None,
                "length".to_string(),
                vec![Expression::value_int(*size as i32)],
            )),
            WindowSpec::Session { value, unit } => {
                let time_expr = Self::create_time_expression(*value, unit)?;
                Ok(stream.window(None, "session".to_string(), vec![time_expr]))
            }
        }
    }

    /// Create time expression from value and unit
    fn create_time_expression(
        value: i64,
        unit: &super::preprocessor::TimeUnit,
    ) -> Result<Expression, ConverterError> {
        use super::preprocessor::TimeUnit;

        let expr = match unit {
            TimeUnit::Milliseconds => Expression::time_millisec(value),
            TimeUnit::Seconds => Expression::time_sec(value),
            TimeUnit::Minutes => Expression::time_minute(value),
            TimeUnit::Hours => Expression::time_hour(value),
        };

        Ok(expr)
    }

    /// Convert SQL expression to EventFlux Expression
    pub fn convert_expression(
        expr: &SqlExpr,
        catalog: &SqlCatalog,
    ) -> Result<Expression, ConverterError> {
        match expr {
            SqlExpr::Identifier(ident) => Ok(Expression::variable(ident.value.clone())),

            SqlExpr::CompoundIdentifier(parts) => {
                // Handle qualified identifiers like stream.column or alias.column
                if parts.len() == 2 {
                    let stream_ref = parts[0].value.clone(); // Stream name or alias (e.g., "t", "n")
                    let column_name = parts[1].value.clone(); // Column name (e.g., "symbol")

                    // Create variable with stream qualifier for JOIN queries
                    let var_with_stream = Variable::new(column_name).of_stream(stream_ref);
                    Ok(Expression::Variable(var_with_stream))
                } else {
                    Err(ConverterError::UnsupportedFeature(
                        "Multi-part identifiers not supported".to_string(),
                    ))
                }
            }

            SqlExpr::Value(value) => match value {
                sqlparser::ast::Value::Number(n, _) => {
                    if n.contains('.') {
                        Ok(Expression::value_double(n.parse().map_err(|_| {
                            ConverterError::InvalidExpression(n.clone())
                        })?))
                    } else {
                        Ok(Expression::value_long(n.parse().map_err(|_| {
                            ConverterError::InvalidExpression(n.clone())
                        })?))
                    }
                }
                sqlparser::ast::Value::SingleQuotedString(s)
                | sqlparser::ast::Value::DoubleQuotedString(s) => {
                    Ok(Expression::value_string(s.clone()))
                }
                sqlparser::ast::Value::Boolean(b) => Ok(Expression::value_bool(*b)),
                _ => Err(ConverterError::UnsupportedFeature(format!(
                    "Value type {:?}",
                    value
                ))),
            },

            SqlExpr::Function(func) => {
                // Convert SQL function calls to EventFlux function calls
                Self::convert_function(func, catalog)
            }

            SqlExpr::BinaryOp { left, op, right } => {
                let left_expr = Self::convert_expression(left, catalog)?;
                let right_expr = Self::convert_expression(right, catalog)?;

                match op {
                    // Comparison operators
                    BinaryOperator::Gt => Ok(Expression::compare(
                        left_expr,
                        CompareOperator::GreaterThan,
                        right_expr,
                    )),
                    BinaryOperator::GtEq => Ok(Expression::compare(
                        left_expr,
                        CompareOperator::GreaterThanEqual,
                        right_expr,
                    )),
                    BinaryOperator::Lt => Ok(Expression::compare(
                        left_expr,
                        CompareOperator::LessThan,
                        right_expr,
                    )),
                    BinaryOperator::LtEq => Ok(Expression::compare(
                        left_expr,
                        CompareOperator::LessThanEqual,
                        right_expr,
                    )),
                    BinaryOperator::Eq => Ok(Expression::compare(
                        left_expr,
                        CompareOperator::Equal,
                        right_expr,
                    )),
                    BinaryOperator::NotEq => Ok(Expression::compare(
                        left_expr,
                        CompareOperator::NotEqual,
                        right_expr,
                    )),

                    // Logical operators
                    BinaryOperator::And => Ok(Expression::and(left_expr, right_expr)),
                    BinaryOperator::Or => Ok(Expression::or(left_expr, right_expr)),

                    // Math operators
                    BinaryOperator::Plus => Ok(Expression::add(left_expr, right_expr)),
                    BinaryOperator::Minus => Ok(Expression::subtract(left_expr, right_expr)),
                    BinaryOperator::Multiply => Ok(Expression::multiply(left_expr, right_expr)),
                    BinaryOperator::Divide => Ok(Expression::divide(left_expr, right_expr)),
                    BinaryOperator::Modulo => Err(ConverterError::UnsupportedFeature(
                        "Modulo operator not yet supported".to_string(),
                    )),

                    _ => Err(ConverterError::UnsupportedFeature(format!(
                        "Binary operator {:?}",
                        op
                    ))),
                }
            }

            SqlExpr::UnaryOp { op, expr } => {
                let inner_expr = Self::convert_expression(expr, catalog)?;

                match op {
                    UnaryOperator::Not => Ok(Expression::not(inner_expr)),
                    _ => Err(ConverterError::UnsupportedFeature(format!(
                        "Unary operator {:?}",
                        op
                    ))),
                }
            }

            _ => Err(ConverterError::UnsupportedFeature(format!(
                "Expression type {:?}",
                expr
            ))),
        }
    }

    /// Convert SQL function to EventFlux function call
    fn convert_function(
        func: &sqlparser::ast::Function,
        catalog: &SqlCatalog,
    ) -> Result<Expression, ConverterError> {
        let func_name = func.name.to_string().to_lowercase();

        // Convert function arguments
        let mut args = Vec::new();
        for arg in &func.args {
            match arg {
                sqlparser::ast::FunctionArg::Unnamed(sqlparser::ast::FunctionArgExpr::Expr(
                    expr,
                )) => {
                    args.push(Self::convert_expression(expr, catalog)?);
                }
                sqlparser::ast::FunctionArg::Unnamed(sqlparser::ast::FunctionArgExpr::Wildcard) => {
                    // Handle COUNT(*) - no arguments needed
                    // EventFlux count() takes no arguments
                }
                _ => {
                    return Err(ConverterError::UnsupportedFeature(format!(
                        "Function argument type not supported"
                    )));
                }
            }
        }

        // Map SQL function names to EventFlux function names
        let eventflux_func_name = match func_name.as_str() {
            "count" => "count",
            "sum" => "sum",
            "avg" => "avg",
            "min" => "min",
            "max" => "max",
            "round" => "round",
            "abs" => "abs",
            "ceil" => "ceil",
            "floor" => "floor",
            "sqrt" => "sqrt",
            "upper" => "upper",
            "lower" => "lower",
            "length" => "length",
            "concat" => "concat",
            _ => {
                return Err(ConverterError::UnsupportedFeature(format!(
                    "Function '{}' not supported in M1",
                    func_name
                )))
            }
        };

        Ok(Expression::function_no_ns(
            eventflux_func_name.to_string(),
            args,
        ))
    }

    /// Convert SQL expression to Constant (for LIMIT/OFFSET)
    fn convert_to_constant(
        expr: &SqlExpr,
    ) -> Result<crate::query_api::expression::constant::Constant, ConverterError> {
        match expr {
            SqlExpr::Value(sqlparser::ast::Value::Number(n, _)) => {
                // Try to parse as i64 for LIMIT/OFFSET
                let num = n.parse::<i64>().map_err(|_| {
                    ConverterError::ConversionFailed(format!(
                        "Invalid number for LIMIT/OFFSET: {}",
                        n
                    ))
                })?;
                Ok(crate::query_api::expression::constant::Constant::long(num))
            }
            _ => Err(ConverterError::UnsupportedFeature(
                "LIMIT/OFFSET must be numeric constants".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query_api::definition::attribute::Type as AttributeType;
    use crate::query_api::definition::StreamDefinition;

    fn setup_catalog() -> SqlCatalog {
        let mut catalog = SqlCatalog::new();
        let stream = StreamDefinition::new("StockStream".to_string())
            .attribute("symbol".to_string(), AttributeType::STRING)
            .attribute("price".to_string(), AttributeType::DOUBLE)
            .attribute("volume".to_string(), AttributeType::INT);

        catalog
            .register_stream("StockStream".to_string(), stream)
            .unwrap();
        catalog
    }

    #[test]
    fn test_simple_select() {
        let catalog = setup_catalog();
        let sql = "SELECT symbol, price FROM StockStream";
        let query = SqlConverter::convert(sql, &catalog).unwrap();

        // Verify query structure
        assert!(query.get_input_stream().is_some());
    }

    #[test]
    fn test_select_with_where() {
        let catalog = setup_catalog();
        let sql = "SELECT symbol, price FROM StockStream WHERE price > 100";
        let query = SqlConverter::convert(sql, &catalog).unwrap();

        assert!(query.get_input_stream().is_some());
    }

    #[test]
    fn test_select_with_window() {
        let catalog = setup_catalog();
        let sql = "SELECT symbol, price FROM StockStream WINDOW LENGTH(5)";
        let query = SqlConverter::convert(sql, &catalog).unwrap();

        assert!(query.get_input_stream().is_some());
    }

    #[test]
    fn test_unknown_stream_error() {
        let catalog = setup_catalog();
        let sql = "SELECT * FROM UnknownStream";
        let result = SqlConverter::convert(sql, &catalog);

        assert!(result.is_err());
    }
}
