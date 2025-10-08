// SPDX-License-Identifier: MIT OR Apache-2.0

//! SELECT Expansion - Expand SELECT * using Schema
//!
//! Expands wildcard SELECT items using schema information from the catalog.

use sqlparser::ast::{Expr as SqlExpr, SelectItem};

use crate::query_api::execution::query::selection::selector::Selector;
use crate::query_api::expression::variable::Variable;
use crate::query_api::expression::Expression;

use super::catalog::SqlCatalog;
use super::converter::SqlConverter;
use super::error::ExpansionError;

/// SELECT Item Expander
pub struct SelectExpander;

impl SelectExpander {
    /// Expand SELECT items using catalog schema
    pub fn expand_select_items(
        items: &[SelectItem],
        from_stream: &str,
        catalog: &SqlCatalog,
    ) -> Result<Selector, ExpansionError> {
        let mut selector = Selector::new();

        for item in items {
            match item {
                SelectItem::Wildcard(_) => {
                    // Expand SELECT * using schema
                    let columns = catalog
                        .get_all_columns(from_stream)
                        .map_err(|_| ExpansionError::UnknownStream(from_stream.to_string()))?;

                    for column in columns {
                        selector =
                            selector.select_variable(Variable::new(column.get_name().to_string()));
                    }
                }

                SelectItem::QualifiedWildcard(kind, _) => {
                    // Handle qualified wildcard: stream.*
                    let object_name = match kind {
                        sqlparser::ast::SelectItemQualifiedWildcardKind::ObjectName(name) => name,
                        sqlparser::ast::SelectItemQualifiedWildcardKind::Expr(_) => {
                            return Err(ExpansionError::UnsupportedFeature(
                                "Expression wildcards not supported".to_string(),
                            ));
                        }
                    };

                    let stream_name = object_name
                        .0
                        .last()
                        .and_then(|part| part.as_ident())
                        .map(|ident| ident.value.as_str())
                        .ok_or_else(|| {
                            ExpansionError::InvalidSelectItem(
                                "Invalid qualified wildcard".to_string(),
                            )
                        })?;

                    let columns = catalog
                        .get_all_columns(stream_name)
                        .map_err(|_| ExpansionError::UnknownStream(stream_name.to_string()))?;

                    for column in columns {
                        selector =
                            selector.select_variable(Variable::new(column.get_name().to_string()));
                    }
                }

                SelectItem::UnnamedExpr(expr) => {
                    // Single expression without alias
                    if let SqlExpr::Identifier(ident) = expr {
                        // Simple column reference
                        let column_name = ident.value.clone();

                        // Validate column exists
                        if !catalog.has_column(from_stream, &column_name) {
                            return Err(ExpansionError::UnknownColumn(
                                from_stream.to_string(),
                                column_name,
                            ));
                        }

                        selector = selector.select_variable(Variable::new(column_name));
                    } else {
                        // Complex expression - convert using SqlConverter
                        let converted_expr = SqlConverter::convert_expression(expr, catalog)
                            .map_err(|e| ExpansionError::InvalidSelectItem(e.to_string()))?;

                        // Generate automatic alias for complex expression
                        let auto_alias = format!("expr_{}", selector.get_selection_list().len());
                        selector = selector.select(auto_alias, converted_expr);
                    }
                }

                SelectItem::ExprWithAlias { expr, alias } => {
                    // Expression with alias
                    let alias_name = alias.value.clone();

                    if let SqlExpr::Identifier(ident) = expr {
                        // Column with alias
                        let column_name = ident.value.clone();

                        // Validate column exists
                        if !catalog.has_column(from_stream, &column_name) {
                            return Err(ExpansionError::UnknownColumn(
                                from_stream.to_string(),
                                column_name,
                            ));
                        }

                        selector = selector.select(alias_name, Expression::variable(column_name));
                    } else {
                        // Complex expression with alias - convert using SqlConverter
                        let converted_expr = SqlConverter::convert_expression(expr, catalog)
                            .map_err(|e| ExpansionError::InvalidSelectItem(e.to_string()))?;

                        selector = selector.select(alias_name, converted_expr);
                    }
                }
            }
        }

        Ok(selector)
    }

    /// Get explicit column list from SELECT items (for schema validation)
    pub fn get_explicit_columns(items: &[SelectItem]) -> Vec<String> {
        let mut columns = Vec::new();

        for item in items {
            match item {
                SelectItem::UnnamedExpr(SqlExpr::Identifier(ident)) => {
                    columns.push(ident.value.clone());
                }
                SelectItem::ExprWithAlias { expr, alias } => {
                    if let SqlExpr::Identifier(ident) = expr {
                        columns.push(ident.value.clone());
                    } else {
                        columns.push(alias.value.clone());
                    }
                }
                _ => {}
            }
        }

        columns
    }

    /// Check if SELECT contains wildcard
    pub fn has_wildcard(items: &[SelectItem]) -> bool {
        items.iter().any(|item| {
            matches!(
                item,
                SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _)
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query_api::definition::attribute::Type as AttributeType;
    use crate::query_api::definition::StreamDefinition;

    fn setup_catalog() -> SqlCatalog {
        let mut catalog = SqlCatalog::new();
        let stream = StreamDefinition::new("TestStream".to_string())
            .attribute("col1".to_string(), AttributeType::STRING)
            .attribute("col2".to_string(), AttributeType::INT)
            .attribute("col3".to_string(), AttributeType::DOUBLE);

        catalog
            .register_stream("TestStream".to_string(), stream)
            .unwrap();
        catalog
    }

    #[test]
    fn test_expand_wildcard() {
        use sqlparser::dialect::GenericDialect;
        use sqlparser::parser::Parser;

        let catalog = setup_catalog();
        let sql = "SELECT * FROM TestStream";
        let statements = Parser::parse_sql(&GenericDialect, sql).unwrap();

        if let sqlparser::ast::Statement::Query(query) = &statements[0] {
            if let sqlparser::ast::SetExpr::Select(select) = query.body.as_ref() {
                let selector =
                    SelectExpander::expand_select_items(&select.projection, "TestStream", &catalog)
                        .unwrap();

                // Should expand to 3 columns
                let output_attrs = selector.get_selection_list();
                assert_eq!(output_attrs.len(), 3);
            }
        }
    }

    #[test]
    fn test_explicit_columns() {
        use sqlparser::dialect::GenericDialect;
        use sqlparser::parser::Parser;

        let catalog = setup_catalog();
        let sql = "SELECT col1, col2 FROM TestStream";
        let statements = Parser::parse_sql(&GenericDialect, sql).unwrap();

        if let sqlparser::ast::Statement::Query(query) = &statements[0] {
            if let sqlparser::ast::SetExpr::Select(select) = query.body.as_ref() {
                let selector =
                    SelectExpander::expand_select_items(&select.projection, "TestStream", &catalog)
                        .unwrap();

                let output_attrs = selector.get_selection_list();
                assert_eq!(output_attrs.len(), 2);
            }
        }
    }

    #[test]
    fn test_has_wildcard() {
        use sqlparser::dialect::GenericDialect;
        use sqlparser::parser::Parser;

        let sql1 = "SELECT * FROM TestStream";
        let statements1 = Parser::parse_sql(&GenericDialect, sql1).unwrap();
        if let sqlparser::ast::Statement::Query(query) = &statements1[0] {
            if let sqlparser::ast::SetExpr::Select(select) = query.body.as_ref() {
                assert!(SelectExpander::has_wildcard(&select.projection));
            }
        }

        let sql2 = "SELECT col1, col2 FROM TestStream";
        let statements2 = Parser::parse_sql(&GenericDialect, sql2).unwrap();
        if let sqlparser::ast::Statement::Query(query) = &statements2[0] {
            if let sqlparser::ast::SetExpr::Select(select) = query.body.as_ref() {
                assert!(!SelectExpander::has_wildcard(&select.projection));
            }
        }
    }

    #[test]
    fn test_unknown_column_error() {
        use sqlparser::dialect::GenericDialect;
        use sqlparser::parser::Parser;

        let catalog = setup_catalog();
        let sql = "SELECT unknown_col FROM TestStream";
        let statements = Parser::parse_sql(&GenericDialect, sql).unwrap();

        if let sqlparser::ast::Statement::Query(query) = &statements[0] {
            if let sqlparser::ast::SetExpr::Select(select) = query.body.as_ref() {
                let result =
                    SelectExpander::expand_select_items(&select.projection, "TestStream", &catalog);
                assert!(result.is_err());
            }
        }
    }
}
