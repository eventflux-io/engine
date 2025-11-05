// SPDX-License-Identifier: MIT OR Apache-2.0

//! SQL to Query Converter
//!
//! Converts SQL statements to EventFlux query_api::Query structures.

use sqlparser::ast::{
    BinaryOperator, Expr as SqlExpr, JoinConstraint, JoinOperator, PartitionKey,
    Select as SqlSelect, SetExpr, Statement, TableFactor, UnaryOperator,
};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

use crate::core::query::processor::stream::window::types::{
    WINDOW_TYPE_EXTERNAL_TIME, WINDOW_TYPE_EXTERNAL_TIME_BATCH, WINDOW_TYPE_LENGTH,
    WINDOW_TYPE_LENGTH_BATCH, WINDOW_TYPE_SESSION, WINDOW_TYPE_SORT, WINDOW_TYPE_TIME,
    WINDOW_TYPE_TIME_BATCH,
};
use crate::query_api::execution::partition::Partition;
use crate::query_api::execution::query::input::stream::input_stream::InputStream;
use crate::query_api::execution::query::input::stream::single_input_stream::SingleInputStream;
use crate::query_api::execution::query::output::output_stream::{
    InsertIntoStreamAction, OutputStream, OutputStreamAction,
};
use crate::query_api::execution::query::Query;
use crate::query_api::expression::variable::Variable;
use crate::query_api::expression::CompareOperator;
use crate::query_api::expression::Expression;

use super::catalog::SqlCatalog;
use super::error::ConverterError;
use super::expansion::SelectExpander;
use super::type_inference::TypeInferenceEngine;

/// SQL to Query Converter
pub struct SqlConverter;

impl SqlConverter {
    /// Convert SQL string to Query (legacy API - parses then converts)
    pub fn convert(sql: &str, catalog: &SqlCatalog) -> Result<Query, ConverterError> {
        // Parse SQL directly (WINDOW clause now handled natively by parser)
        let statements = Parser::parse_sql(&GenericDialect, sql)
            .map_err(|e| ConverterError::ConversionFailed(format!("SQL parse error: {}", e)))?;

        if statements.is_empty() {
            return Err(ConverterError::ConversionFailed(
                "No SQL statements found".to_string(),
            ));
        }

        // Convert SELECT or INSERT INTO statement to Query
        match &statements[0] {
            Statement::Query(query) => Self::convert_query_ast(query, catalog, None),
            Statement::Insert(insert) => {
                let target_stream = match &insert.table {
                    sqlparser::ast::TableObject::TableName(name) => name.to_string(),
                    sqlparser::ast::TableObject::TableFunction(_) => {
                        return Err(ConverterError::UnsupportedFeature(
                            "Table functions not supported in INSERT".to_string(),
                        ))
                    }
                };

                let source = insert.source.as_ref().ok_or_else(|| {
                    ConverterError::UnsupportedFeature(
                        "INSERT without SELECT source not supported".to_string(),
                    )
                })?;

                Self::convert_query_ast(source, catalog, Some(target_stream))
            }
            _ => Err(ConverterError::UnsupportedFeature(
                "Only SELECT and INSERT INTO queries are supported".to_string(),
            )),
        }
    }

    /// Convert parsed Query AST directly to Query (no re-parsing!)
    ///
    /// This is the preferred method when you already have a parsed AST.
    /// It avoids the overhead of serializing and re-parsing the AST.
    pub fn convert_query_ast(
        query: &sqlparser::ast::Query,
        catalog: &SqlCatalog,
        output_stream_name: Option<String>,
    ) -> Result<Query, ConverterError> {
        Self::convert_query_internal(query, catalog, output_stream_name)
    }

    /// Convert PARTITION statement to Partition execution element
    pub fn convert_partition(
        partition_keys: &[PartitionKey],
        body: &[Statement],
        catalog: &SqlCatalog,
    ) -> Result<Partition, ConverterError> {
        let mut partition = Partition::new();

        // Validate and convert partition keys
        for key in partition_keys {
            let stream_name = key.stream_name.to_string();
            let attribute_name = key.attribute.value.clone();

            // Validate relation (stream or table) exists
            catalog
                .get_relation(&stream_name)
                .map_err(|_| ConverterError::SchemaNotFound(stream_name.clone()))?;

            // Validate attribute exists in the relation
            if !catalog.has_column(&stream_name, &attribute_name) {
                return Err(ConverterError::InvalidExpression(format!(
                    "Attribute '{}' not found in relation '{}'",
                    attribute_name, stream_name
                )));
            }

            // Create partition expression
            let partition_expr = Expression::Variable(Variable::new(attribute_name.clone()));
            partition = partition.with_value_partition(stream_name, partition_expr);
        }

        // Convert body statements to queries
        for stmt in body {
            match stmt {
                Statement::Query(query) => {
                    let q = Self::convert_query_ast(query, catalog, None)?;
                    partition = partition.add_query(q);
                }
                Statement::Insert(insert) => {
                    let target_stream = match &insert.table {
                        sqlparser::ast::TableObject::TableName(name) => name.to_string(),
                        sqlparser::ast::TableObject::TableFunction(_) => {
                            return Err(ConverterError::UnsupportedFeature(
                                "Table functions not supported in INSERT".to_string(),
                            ))
                        }
                    };

                    let source = insert.source.as_ref().ok_or_else(|| {
                        ConverterError::UnsupportedFeature(
                            "INSERT without SELECT source not supported".to_string(),
                        )
                    })?;

                    let q = Self::convert_query_ast(source, catalog, Some(target_stream))?;
                    partition = partition.add_query(q);
                }
                _ => {
                    return Err(ConverterError::UnsupportedFeature(
                        "Only SELECT and INSERT INTO statements supported inside PARTITION"
                            .to_string(),
                    ))
                }
            }
        }

        Ok(partition)
    }

    /// Internal method to convert sqlparser Query AST to EventFlux Query
    fn convert_query_internal(
        sql_query: &sqlparser::ast::Query,
        catalog: &SqlCatalog,
        output_stream_name: Option<String>,
    ) -> Result<Query, ConverterError> {
        // Extract limit and offset from limit_clause
        let (limit, offset) = match &sql_query.limit_clause {
            Some(sqlparser::ast::LimitClause::LimitOffset { limit, offset, .. }) => {
                (limit.as_ref(), offset.as_ref())
            }
            Some(sqlparser::ast::LimitClause::OffsetCommaLimit { .. }) => {
                return Err(ConverterError::UnsupportedFeature(
                    "MySQL-style LIMIT offset,limit syntax not supported".to_string(),
                ))
            }
            None => (None, None),
        };

        match sql_query.body.as_ref() {
            SetExpr::Select(select) => Self::convert_select(
                select,
                catalog,
                sql_query.order_by.as_ref(),
                limit,
                offset,
                output_stream_name,
            ),
            _ => Err(ConverterError::UnsupportedFeature(
                "Only simple SELECT supported".to_string(),
            )),
        }
    }

    /// Convert SELECT statement to Query
    fn convert_select(
        select: &SqlSelect,
        catalog: &SqlCatalog,
        order_by: Option<&sqlparser::ast::OrderBy>,
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

            // Validate relation (stream or table) exists
            catalog
                .get_relation(&stream_name)
                .map_err(|_| ConverterError::SchemaNotFound(stream_name.clone()))?;

            // Create InputStream (works for both streams and tables - runtime will differentiate)
            let mut single_stream = SingleInputStream::new_basic(
                stream_name.clone(),
                false,      // is_inner_stream
                false,      // is_fault_stream
                None,       // stream_handler_id
                Vec::new(), // pre_window_handlers
            );

            // Add WINDOW if present from AST
            if let Some(window_ast) = Self::extract_window_from_table_factor(&select.from) {
                single_stream = Self::add_window_from_ast(single_stream, window_ast, catalog)?;
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
        if let sqlparser::ast::GroupByExpr::Expressions(group_exprs, modifiers) = &select.group_by {
            if !modifiers.is_empty() {
                return Err(ConverterError::UnsupportedFeature(
                    "GROUP BY modifiers (ROLLUP, CUBE, etc.) not supported".to_string(),
                ));
            }

            for expr in group_exprs {
                if let SqlExpr::Identifier(ident) = expr {
                    selector = selector.group_by(Variable::new(ident.value.clone()));
                } else {
                    return Err(ConverterError::UnsupportedFeature(
                        "Complex GROUP BY expressions not supported".to_string(),
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
        if let Some(order_by) = order_by {
            // Extract expressions from OrderBy
            let order_exprs = match &order_by.kind {
                sqlparser::ast::OrderByKind::Expressions(exprs) => exprs,
                sqlparser::ast::OrderByKind::All(_) => {
                    return Err(ConverterError::UnsupportedFeature(
                        "ORDER BY ALL not supported".to_string(),
                    ))
                }
            };

            for order_expr in order_exprs {
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
                            "Complex expressions in ORDER BY not supported".to_string(),
                        ))
                    }
                };

                // Determine order (ASC/DESC)
                let order = if let Some(asc) = order_expr.options.asc {
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
        let query = Query::query()
            .from(input_stream)
            .select(selector)
            .out_stream(output_stream);

        // Validate query for type correctness (no allocation, uses catalog reference)
        let type_engine = TypeInferenceEngine::new(catalog);
        type_engine.validate_query(&query).map_err(|e| {
            ConverterError::ConversionFailed(format!("Type validation failed: {}", e))
        })?;

        Ok(query)
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
                .and_then(|part| part.as_ident())
                .map(|ident| ident.value.clone())
                .ok_or_else(|| {
                    ConverterError::ConversionFailed("No table name in FROM".to_string())
                }),
            _ => Err(ConverterError::UnsupportedFeature(
                "Complex FROM clauses not supported".to_string(),
            )),
        }
    }

    /// Extract window specification from TableFactor (native AST field)
    fn extract_window_from_table_factor(
        from: &[sqlparser::ast::TableWithJoins],
    ) -> Option<&sqlparser::ast::StreamingWindowSpec> {
        if from.is_empty() {
            return None;
        }

        match &from[0].relation {
            TableFactor::Table { window, .. } => window.as_ref(),
            _ => None,
        }
    }

    /// Convert JOIN from clause to JoinInputStream
    fn convert_join_from_clause(
        from: &[sqlparser::ast::TableWithJoins],
        _where_clause: &Option<SqlExpr>, // Reserved for future filter optimization
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
                let stream_name = name
                    .0
                    .last()
                    .and_then(|part| part.as_ident())
                    .map(|ident| ident.value.clone())
                    .ok_or_else(|| {
                        ConverterError::ConversionFailed("No left table name".to_string())
                    })?;

                // Validate relation (stream or table) exists for JOIN
                let _ = catalog
                    .get_relation(&stream_name)
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

        // Get first JOIN (only support single JOIN currently)
        let join = &from[0].joins[0];

        // Extract right stream
        let right_stream_name = match &join.relation {
            TableFactor::Table { name, alias, .. } => {
                let stream_name = name
                    .0
                    .last()
                    .and_then(|part| part.as_ident())
                    .map(|ident| ident.value.clone())
                    .ok_or_else(|| {
                        ConverterError::ConversionFailed("No right table name".to_string())
                    })?;

                // Validate relation (stream or table) exists for JOIN
                let _ = catalog
                    .get_relation(&stream_name)
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

        // Extract join type and ON condition together
        // Note: In SQL, plain JOIN and INNER JOIN are identical (ANSI standard),
        // so we normalize them to InnerJoin for consistency
        let (join_type, on_condition) = match &join.join_operator {
            // INNER JOIN variants (normalize plain JOIN to INNER JOIN)
            JoinOperator::Join(constraint) | JoinOperator::Inner(constraint) => {
                let cond = Self::extract_on_condition(constraint, catalog)?;
                (JoinType::InnerJoin, cond)
            }
            // OUTER JOIN variants
            JoinOperator::LeftOuter(constraint) => {
                let cond = Self::extract_on_condition(constraint, catalog)?;
                (JoinType::LeftOuterJoin, cond)
            }
            JoinOperator::RightOuter(constraint) => {
                let cond = Self::extract_on_condition(constraint, catalog)?;
                (JoinType::RightOuterJoin, cond)
            }
            JoinOperator::FullOuter(constraint) => {
                let cond = Self::extract_on_condition(constraint, catalog)?;
                (JoinType::FullOuterJoin, cond)
            }
            _ => {
                return Err(ConverterError::UnsupportedFeature(format!(
                    "Unsupported join operator: {:?}",
                    join.join_operator
                )))
            }
        };

        // Create JoinInputStream
        let join_stream = JoinInputStream::new(
            left_stream_name,
            join_type,
            right_stream_name,
            on_condition,
            EventTrigger::All, // Default trigger
            None,              // No WITHIN clause currently
            None,              // No PER clause currently
        );

        Ok(InputStream::Join(Box::new(join_stream)))
    }

    /// Extract ON condition from JoinConstraint
    /// Currently only supports ON clause; USING and NATURAL joins are not yet implemented
    fn extract_on_condition(
        constraint: &JoinConstraint,
        catalog: &SqlCatalog,
    ) -> Result<Option<Expression>, ConverterError> {
        match constraint {
            JoinConstraint::On(expr) => Ok(Some(Self::convert_expression(expr, catalog)?)),
            JoinConstraint::Using(_) => Err(ConverterError::UnsupportedFeature(
                "JOIN USING clause not yet supported. Use ON clause instead.".to_string(),
            )),
            JoinConstraint::Natural => Err(ConverterError::UnsupportedFeature(
                "NATURAL JOIN not yet supported. Use explicit ON clause.".to_string(),
            )),
            JoinConstraint::None => Ok(None),
        }
    }

    /// Add window to SingleInputStream
    /// Add window from native AST StreamingWindowSpec
    fn add_window_from_ast(
        stream: SingleInputStream,
        window: &sqlparser::ast::StreamingWindowSpec,
        catalog: &SqlCatalog,
    ) -> Result<SingleInputStream, ConverterError> {
        use sqlparser::ast::StreamingWindowSpec;

        match window {
            StreamingWindowSpec::Tumbling { duration } => {
                // Tumbling windows are non-overlapping time-based batches
                let duration_expr = Self::convert_expression(duration, catalog)?;
                Ok(stream.window(
                    None,
                    WINDOW_TYPE_TIME_BATCH.to_string(),
                    vec![duration_expr],
                ))
            }
            StreamingWindowSpec::Sliding { size, slide } => {
                // Sliding/hopping windows not yet implemented
                // TODO: Implement sliding window processor (requires size + slide parameters)
                let _size_expr = Self::convert_expression(size, catalog)?;
                let _slide_expr = Self::convert_expression(slide, catalog)?;
                Err(ConverterError::UnsupportedFeature(
                    "Sliding windows not yet implemented. Use 'time' for overlapping windows or 'timeBatch' for non-overlapping.".to_string()
                ))
            }
            StreamingWindowSpec::Length { size } => {
                let size_expr = Self::convert_expression(size, catalog)?;
                Ok(stream.window(None, WINDOW_TYPE_LENGTH.to_string(), vec![size_expr]))
            }
            StreamingWindowSpec::Session { gap } => {
                let gap_expr = Self::convert_expression(gap, catalog)?;
                Ok(stream.window(None, WINDOW_TYPE_SESSION.to_string(), vec![gap_expr]))
            }
            StreamingWindowSpec::Time { duration } => {
                let duration_expr = Self::convert_expression(duration, catalog)?;
                Ok(stream.window(None, WINDOW_TYPE_TIME.to_string(), vec![duration_expr]))
            }
            StreamingWindowSpec::TimeBatch { duration } => {
                let duration_expr = Self::convert_expression(duration, catalog)?;
                Ok(stream.window(
                    None,
                    WINDOW_TYPE_TIME_BATCH.to_string(),
                    vec![duration_expr],
                ))
            }
            StreamingWindowSpec::LengthBatch { size } => {
                let size_expr = Self::convert_expression(size, catalog)?;
                Ok(stream.window(None, WINDOW_TYPE_LENGTH_BATCH.to_string(), vec![size_expr]))
            }
            StreamingWindowSpec::ExternalTime {
                timestamp_field,
                duration,
            } => {
                let ts_expr = Self::convert_expression(timestamp_field, catalog)?;
                let duration_expr = Self::convert_expression(duration, catalog)?;
                Ok(stream.window(
                    None,
                    WINDOW_TYPE_EXTERNAL_TIME.to_string(),
                    vec![ts_expr, duration_expr],
                ))
            }
            StreamingWindowSpec::ExternalTimeBatch {
                timestamp_field,
                duration,
            } => {
                let ts_expr = Self::convert_expression(timestamp_field, catalog)?;
                let duration_expr = Self::convert_expression(duration, catalog)?;
                Ok(stream.window(
                    None,
                    WINDOW_TYPE_EXTERNAL_TIME_BATCH.to_string(),
                    vec![ts_expr, duration_expr],
                ))
            }
            StreamingWindowSpec::Sort { parameters } => {
                // Convert all parameters (size, attr1, 'asc', attr2, 'desc', ...)
                let mut converted_params = Vec::new();
                for param in parameters {
                    converted_params.push(Self::convert_expression(param, catalog)?);
                }
                Ok(stream.window(None, WINDOW_TYPE_SORT.to_string(), converted_params))
            }
        }
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

            SqlExpr::Value(value_with_span) => match &value_with_span.value {
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
                    value_with_span.value
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

            SqlExpr::Interval(interval) => {
                // Convert INTERVAL '5' SECOND to milliseconds
                Self::convert_interval_to_millis(interval)
            }

            _ => Err(ConverterError::UnsupportedFeature(format!(
                "Expression type {:?}",
                expr
            ))),
        }
    }

    /// Convert SQL INTERVAL to milliseconds as a Long expression
    ///
    /// # Approximations
    ///
    /// Note that YEAR and MONTH intervals use fixed approximations:
    /// - 1 YEAR = 365 days (does not account for leap years)
    /// - 1 MONTH = 30 days (months vary from 28-31 days)
    ///
    /// For precise time-based window operations, prefer SECOND, MINUTE, HOUR, or DAY units.
    ///
    /// # Example
    ///
    /// ```sql
    /// INTERVAL '5' SECOND  -- Exact: 5000 milliseconds
    /// INTERVAL '1' MONTH   -- Approximation: 2592000000 milliseconds (30 days)
    /// ```
    fn convert_interval_to_millis(
        interval: &sqlparser::ast::Interval,
    ) -> Result<Expression, ConverterError> {
        // Extract the numeric value
        let value = match interval.value.as_ref() {
            SqlExpr::Value(value_with_span) => match &value_with_span.value {
                sqlparser::ast::Value::Number(n, _) => n.parse::<i64>().map_err(|_| {
                    ConverterError::InvalidExpression(format!("Invalid interval value: {}", n))
                })?,
                sqlparser::ast::Value::SingleQuotedString(s) => s.parse::<i64>().map_err(|_| {
                    ConverterError::InvalidExpression(format!("Invalid interval value: {}", s))
                })?,
                _ => {
                    return Err(ConverterError::UnsupportedFeature(
                        "Complex interval values not supported".to_string(),
                    ))
                }
            },
            _ => {
                return Err(ConverterError::UnsupportedFeature(
                    "Complex interval expressions not supported".to_string(),
                ))
            }
        };

        // Convert based on time unit
        let millis = match &interval.leading_field {
            Some(sqlparser::ast::DateTimeField::Year) => {
                // Approximation: 365 days (ignores leap years)
                value * 365 * 24 * 60 * 60 * 1000
            }
            Some(sqlparser::ast::DateTimeField::Month) => {
                // Approximation: 30 days (months vary 28-31 days)
                value * 30 * 24 * 60 * 60 * 1000
            }
            Some(sqlparser::ast::DateTimeField::Day) => value * 24 * 60 * 60 * 1000,
            Some(sqlparser::ast::DateTimeField::Hour) => value * 60 * 60 * 1000,
            Some(sqlparser::ast::DateTimeField::Minute) => value * 60 * 1000,
            Some(sqlparser::ast::DateTimeField::Second) => value * 1000,
            None => {
                // Default to milliseconds if no unit specified
                value
            }
            _ => {
                return Err(ConverterError::UnsupportedFeature(format!(
                    "Interval unit {:?} not supported",
                    interval.leading_field
                )))
            }
        };

        Ok(Expression::value_long(millis))
    }

    /// Convert SQL function to EventFlux function call
    fn convert_function(
        func: &sqlparser::ast::Function,
        catalog: &SqlCatalog,
    ) -> Result<Expression, ConverterError> {
        let func_name = func.name.to_string().to_lowercase();

        // Extract function argument list
        let arg_list = match &func.args {
            sqlparser::ast::FunctionArguments::List(list) => list,
            sqlparser::ast::FunctionArguments::None => {
                // Functions like CURRENT_TIMESTAMP with no args
                return Ok(Expression::function(None, func_name, Vec::new()));
            }
            sqlparser::ast::FunctionArguments::Subquery(_) => {
                return Err(ConverterError::UnsupportedFeature(
                    "Subquery as function argument not supported".to_string(),
                ));
            }
        };

        // Convert function arguments
        let mut args = Vec::new();
        for arg in &arg_list.args {
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
                    return Err(ConverterError::UnsupportedFeature(
                        "Function argument type not supported".to_string(),
                    ));
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
                    "Function '{}' not supported",
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
            SqlExpr::Value(value_with_span) => {
                if let sqlparser::ast::Value::Number(n, _) = &value_with_span.value {
                    // Try to parse as i64 for LIMIT/OFFSET
                    let num = n.parse::<i64>().map_err(|_| {
                        ConverterError::ConversionFailed(format!(
                            "Invalid number for LIMIT/OFFSET: {}",
                            n
                        ))
                    })?;
                    Ok(crate::query_api::expression::constant::Constant::long(num))
                } else {
                    Err(ConverterError::UnsupportedFeature(
                        "LIMIT/OFFSET must be numeric constants".to_string(),
                    ))
                }
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
        let sql = "SELECT symbol, price FROM StockStream WINDOW('length', 5)";
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

    // ============================================================================
    // Type Validation Error Tests
    // ============================================================================

    #[test]
    fn test_where_clause_non_boolean_variable() {
        let catalog = setup_catalog();
        // WHERE price - returns DOUBLE, not BOOL
        let sql = "SELECT symbol, price FROM StockStream WHERE price";
        let result = SqlConverter::convert(sql, &catalog);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Type validation failed"),
            "Error message should mention type validation: {}",
            err_msg
        );
        assert!(
            err_msg.contains("WHERE"),
            "Error message should mention WHERE clause: {}",
            err_msg
        );
    }

    #[test]
    fn test_where_clause_arithmetic_expression() {
        let catalog = setup_catalog();
        // WHERE price * 2 - returns DOUBLE, not BOOL
        let sql = "SELECT symbol, price FROM StockStream WHERE price * 2";
        let result = SqlConverter::convert(sql, &catalog);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Type validation failed"),
            "Error message should mention type validation: {}",
            err_msg
        );
        assert!(
            err_msg.contains("WHERE"),
            "Error message should mention WHERE clause: {}",
            err_msg
        );
    }

    #[test]
    fn test_where_clause_valid_boolean() {
        let catalog = setup_catalog();
        // WHERE price > 100 - valid BOOL expression
        let sql = "SELECT symbol, price FROM StockStream WHERE price > 100";
        let result = SqlConverter::convert(sql, &catalog);

        assert!(
            result.is_ok(),
            "Valid WHERE clause should succeed: {:?}",
            result.unwrap_err()
        );
    }

    #[test]
    fn test_having_clause_non_boolean() {
        let catalog = setup_catalog();
        // HAVING SUM(volume) - returns LONG, not BOOL
        let sql = "SELECT symbol, SUM(volume) as total FROM StockStream GROUP BY symbol HAVING SUM(volume)";
        let result = SqlConverter::convert(sql, &catalog);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Type validation failed"),
            "Error message should mention type validation: {}",
            err_msg
        );
        assert!(
            err_msg.contains("HAVING"),
            "Error message should mention HAVING clause: {}",
            err_msg
        );
    }

    #[test]
    fn test_having_clause_valid_boolean() {
        let catalog = setup_catalog();
        // HAVING SUM(volume) > 1000 - valid BOOL expression
        let sql = "SELECT symbol, SUM(volume) as total FROM StockStream GROUP BY symbol HAVING SUM(volume) > 1000";
        let result = SqlConverter::convert(sql, &catalog);

        assert!(
            result.is_ok(),
            "Valid HAVING clause should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_where_with_function_non_boolean() {
        let catalog = setup_catalog();
        // WHERE ROUND(price, 2) - returns DOUBLE, not BOOL
        let sql = "SELECT symbol, price FROM StockStream WHERE ROUND(price, 2)";
        let result = SqlConverter::convert(sql, &catalog);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Type validation failed"),
            "Error message should mention type validation: {}",
            err_msg
        );
    }
}
