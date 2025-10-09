// SPDX-License-Identifier: MIT OR Apache-2.0

//! Application Parser - Parse Complete SQL Applications
//!
//! Parses multi-statement SQL applications with DDL and queries.

use super::catalog::{SqlApplication, SqlCatalog};
use super::converter::SqlConverter;
use super::ddl::DdlParser;
use super::error::ApplicationError;

/// Parse a complete SQL application with multiple statements
pub fn parse_sql_application(sql: &str) -> Result<SqlApplication, ApplicationError> {
    use sqlparser::dialect::GenericDialect;
    use sqlparser::parser::Parser;
    use crate::sql_compiler::type_mapping::sql_type_to_attribute_type;

    let mut catalog = SqlCatalog::new();
    let mut execution_elements = Vec::new();

    // Replace CREATE STREAM with CREATE TABLE for sqlparser compatibility
    let normalized_sql = sql
        .replace("CREATE STREAM", "CREATE TABLE")
        .replace("create stream", "CREATE TABLE");

    // Parse all statements at once using sqlparser
    let parsed_statements = Parser::parse_sql(&GenericDialect, &normalized_sql)
        .map_err(|e| ApplicationError::Converter(super::error::ConverterError::ConversionFailed(format!("SQL parse error: {}", e))))?;

    if parsed_statements.is_empty() {
        return Err(ApplicationError::EmptyApplication);
    }

    // Process each parsed statement
    for stmt in parsed_statements {
        match stmt {
            sqlparser::ast::Statement::CreateTable(create) => {
                // Handle CREATE STREAM (parsed as CREATE TABLE by sqlparser)
                let stream_name = create.name.to_string();
                let mut stream_def = crate::query_api::definition::StreamDefinition::new(stream_name.clone());

                for col in &create.columns {
                    let attr_type = sql_type_to_attribute_type(&col.data_type)
                        .map_err(|e| ApplicationError::Ddl(super::error::DdlError::InvalidCreateStream(e.to_string())))?;
                    stream_def = stream_def.attribute(col.name.value.clone(), attr_type);
                }

                catalog.register_stream(stream_name, stream_def)?;
            }
            sqlparser::ast::Statement::Query(_) | sqlparser::ast::Statement::Insert(_) => {
                // Convert to execution element
                let sql_text = stmt.to_string();
                let elem = SqlConverter::convert_to_execution_element(&sql_text, &catalog)?;
                execution_elements.push(elem);
            }
            sqlparser::ast::Statement::Partition { partition_keys, body } => {
                // Handle partition directly without re-parsing
                let partition = SqlConverter::convert_partition(&partition_keys, &body, &catalog)?;
                execution_elements.push(crate::query_api::execution::ExecutionElement::Partition(partition));
            }
            _ => {
                return Err(ApplicationError::Converter(super::error::ConverterError::UnsupportedFeature(format!(
                    "Unsupported statement type: {}",
                    stmt
                ))))
            }
        }
    }

    Ok(SqlApplication::new(catalog, execution_elements))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_application() {
        let sql = r#"
            CREATE STREAM StockStream (symbol VARCHAR, price DOUBLE);

            SELECT symbol, price
            FROM StockStream
            WHERE price > 100;
        "#;

        let app = parse_sql_application(sql).unwrap();
        assert!(!app.catalog.is_empty());
        assert_eq!(app.execution_elements.len(), 1);
    }

    #[test]
    fn test_parse_multiple_queries() {
        let sql = r#"
            CREATE STREAM Input1 (x INT);
            CREATE STREAM Input2 (y INT);

            SELECT x FROM Input1;
            SELECT y FROM Input2;
        "#;

        let app = parse_sql_application(sql).unwrap();
        assert_eq!(app.catalog.get_stream_names().len(), 2);
        assert_eq!(app.execution_elements.len(), 2);
    }

    #[test]
    fn test_parse_with_window() {
        let sql = r#"
            CREATE STREAM SensorStream (temp DOUBLE);

            SELECT temp
            FROM SensorStream
            WINDOW('length', 10);
        "#;

        let app = parse_sql_application(sql).unwrap();
        assert_eq!(app.execution_elements.len(), 1);
    }

    #[test]
    fn test_empty_application_error() {
        let sql = "";
        let result = parse_sql_application(sql);
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_stream_in_query() {
        let sql = r#"
            CREATE STREAM Known (x INT);
            SELECT y FROM Unknown;
        "#;

        let result = parse_sql_application(sql);
        assert!(result.is_err());
    }

    #[test]
    fn test_select_wildcard() {
        let sql = r#"
            CREATE STREAM AllColumns (a INT, b DOUBLE, c VARCHAR);
            SELECT * FROM AllColumns;
        "#;

        let app = parse_sql_application(sql).unwrap();
        assert_eq!(app.execution_elements.len(), 1);
    }
}
