// SPDX-License-Identifier: MIT OR Apache-2.0

//! Application Parser - Parse Complete SQL Applications
//!
//! Parses multi-statement SQL applications with DDL and queries.

use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

use super::catalog::{SqlApplication, SqlCatalog};
use super::converter::SqlConverter;
use super::error::ApplicationError;
use super::normalization::normalize_stream_syntax;
use super::type_mapping::sql_type_to_attribute_type;
use super::with_clause::{extract_with_options, validate_with_clause};

/// Parse a complete SQL application with multiple statements
pub fn parse_sql_application(sql: &str) -> Result<SqlApplication, ApplicationError> {
    let mut catalog = SqlCatalog::new();
    let mut execution_elements = Vec::new();

    // Normalize EventFlux-specific syntax for standard SQL parsing
    let normalized_sql = normalize_stream_syntax(sql);

    // Parse all statements at once using sqlparser
    let parsed_statements = Parser::parse_sql(&GenericDialect, &normalized_sql).map_err(|e| {
        ApplicationError::Converter(super::error::ConverterError::ConversionFailed(format!(
            "SQL parse error: {}",
            e
        )))
    })?;

    if parsed_statements.is_empty() {
        return Err(ApplicationError::EmptyApplication);
    }

    // Process each parsed statement
    for stmt in parsed_statements {
        match stmt {
            sqlparser::ast::Statement::CreateTable(create) => {
                // Handle CREATE STREAM (parsed as CREATE TABLE by sqlparser)
                let stream_name = create.name.to_string();
                let mut stream_def =
                    crate::query_api::definition::StreamDefinition::new(stream_name.clone());

                // Extract column definitions
                for col in &create.columns {
                    let attr_type = sql_type_to_attribute_type(&col.data_type)?;
                    stream_def = stream_def.attribute(col.name.value.clone(), attr_type);
                }

                // Extract and validate WITH clause options
                let with_config = extract_with_options(&create.table_options)?;
                if !with_config.is_empty() {
                    // Validate the WITH clause configuration
                    validate_with_clause(&with_config)?;

                    // Store configuration with stream definition for later use
                    // Note: StreamDefinition doesn't currently store FlatConfig,
                    // but the configuration has been validated at parse-time.
                    // Future enhancement: Store with_config in StreamDefinition
                }

                catalog.register_stream(stream_name, stream_def)?;
            }
            sqlparser::ast::Statement::Query(query) => {
                // Convert query AST directly (no re-parsing!)
                let q = SqlConverter::convert_query_ast(&query, &catalog, None)?;
                execution_elements.push(crate::query_api::execution::ExecutionElement::Query(q));
            }
            sqlparser::ast::Statement::Insert(insert) => {
                // Convert INSERT AST directly (no re-parsing!)
                let target_stream = match &insert.table {
                    sqlparser::ast::TableObject::TableName(name) => name.to_string(),
                    sqlparser::ast::TableObject::TableFunction(_) => {
                        return Err(ApplicationError::Converter(
                            super::error::ConverterError::UnsupportedFeature(
                                "Table functions not supported in INSERT".to_string(),
                            ),
                        ))
                    }
                };

                let source = insert.source.as_ref().ok_or_else(|| {
                    ApplicationError::Converter(super::error::ConverterError::UnsupportedFeature(
                        "INSERT without SELECT source not supported".to_string(),
                    ))
                })?;

                let q = SqlConverter::convert_query_ast(source, &catalog, Some(target_stream))?;
                execution_elements.push(crate::query_api::execution::ExecutionElement::Query(q));
            }
            sqlparser::ast::Statement::Partition {
                partition_keys,
                body,
            } => {
                // Handle partition directly without re-parsing
                let partition = SqlConverter::convert_partition(&partition_keys, &body, &catalog)?;
                execution_elements.push(crate::query_api::execution::ExecutionElement::Partition(
                    partition,
                ));
            }
            _ => {
                return Err(ApplicationError::Converter(
                    super::error::ConverterError::UnsupportedFeature(format!(
                        "Unsupported statement type: {}",
                        stmt
                    )),
                ))
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

    // ========================================================================
    // Integration Tests for WITH Clause (M1 Milestone)
    // ========================================================================

    #[test]
    fn test_create_stream_with_source_config() {
        let sql = r#"
            CREATE STREAM DataStream (id INT, value DOUBLE)
            WITH (
                type = 'source',
                extension = 'kafka',
                format = 'json'
            );
        "#;

        let app = parse_sql_application(sql);
        assert!(
            app.is_ok(),
            "Failed to parse source stream with WITH clause: {:?}",
            app.err()
        );

        let app = app.unwrap();
        assert!(!app.catalog.is_empty());
        assert!(app.catalog.get_stream("DataStream").is_ok());
    }

    #[test]
    fn test_create_stream_with_sink_config() {
        let sql = r#"
            CREATE STREAM OutputStream (result VARCHAR, count INT)
            WITH (
                type = 'sink',
                extension = 'log',
                format = 'text'
            );
        "#;

        let app = parse_sql_application(sql);
        assert!(
            app.is_ok(),
            "Failed to parse sink stream with WITH clause: {:?}",
            app.err()
        );

        let app = app.unwrap();
        assert!(app.catalog.get_stream("OutputStream").is_ok());
    }

    #[test]
    fn test_create_stream_with_internal_config() {
        let sql = r#"
            CREATE STREAM TempStream (id INT, value DOUBLE)
            WITH (
                type = 'internal'
            );
        "#;

        let app = parse_sql_application(sql);
        assert!(
            app.is_ok(),
            "Failed to parse internal stream with WITH clause: {:?}",
            app.err()
        );
    }

    #[test]
    fn test_create_stream_without_with_clause() {
        let sql = r#"
            CREATE STREAM SimpleStream (x INT, y DOUBLE);
        "#;

        let app = parse_sql_application(sql);
        assert!(app.is_ok(), "Failed to parse stream without WITH clause");
    }

    #[test]
    fn test_create_stream_with_validation_error_missing_extension() {
        let sql = r#"
            CREATE STREAM BadStream (id INT)
            WITH (
                type = 'source',
                format = 'json'
            );
        "#;

        let result = parse_sql_application(sql);
        assert!(result.is_err(), "Should fail validation: missing extension");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("extension"),
            "Error should mention missing extension: {}",
            err
        );
    }

    #[test]
    fn test_create_stream_with_validation_error_missing_format() {
        let sql = r#"
            CREATE STREAM BadStream (id INT)
            WITH (
                type = 'source',
                extension = 'kafka'
            );
        "#;

        let result = parse_sql_application(sql);
        assert!(result.is_err(), "Should fail validation: missing format");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("format"),
            "Error should mention missing format: {}",
            err
        );
    }

    #[test]
    fn test_create_stream_with_validation_error_internal_with_extension() {
        let sql = r#"
            CREATE STREAM BadStream (id INT)
            WITH (
                type = 'internal',
                extension = 'kafka'
            );
        "#;

        let result = parse_sql_application(sql);
        assert!(
            result.is_err(),
            "Should fail validation: internal stream with extension"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("internal") && err.contains("extension"),
            "Error should mention internal + extension conflict: {}",
            err
        );
    }

    #[test]
    fn test_create_stream_with_validation_error_internal_with_format() {
        let sql = r#"
            CREATE STREAM BadStream (id INT)
            WITH (
                type = 'internal',
                format = 'json'
            );
        "#;

        let result = parse_sql_application(sql);
        assert!(
            result.is_err(),
            "Should fail validation: internal stream with format"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("internal") && err.contains("format"),
            "Error should mention internal + format conflict: {}",
            err
        );
    }

    #[test]
    fn test_create_stream_with_validation_error_invalid_type() {
        let sql = r#"
            CREATE STREAM BadStream (id INT)
            WITH (
                type = 'invalid_type'
            );
        "#;

        let result = parse_sql_application(sql);
        assert!(
            result.is_err(),
            "Should fail validation: invalid stream type"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Invalid stream type"),
            "Error should mention invalid type: {}",
            err
        );
    }

    #[test]
    fn test_integration_create_stream_with_query() {
        let sql = r#"
            CREATE STREAM DataStream (id INT, value DOUBLE)
            WITH (
                type = 'source',
                extension = 'kafka',
                format = 'json'
            );

            SELECT id, value
            FROM DataStream
            WHERE value > 100;
        "#;

        let app = parse_sql_application(sql);
        assert!(
            app.is_ok(),
            "Failed to parse application with stream + query: {:?}",
            app.err()
        );

        let app = app.unwrap();
        assert_eq!(app.catalog.get_stream_names().len(), 1);
        assert_eq!(app.execution_elements.len(), 1);
    }

    #[test]
    fn test_integration_merge_rust_defaults_with_sql_with() {
        // This test demonstrates the multi-layer configuration concept
        // In a real application, Rust defaults and TOML configs would be merged
        // with SQL WITH clause options (highest priority)

        let sql = r#"
            CREATE STREAM DataStream (id INT, value DOUBLE)
            WITH (
                type = 'source',
                extension = 'http',
                format = 'json'
            );
        "#;

        let app = parse_sql_application(sql);
        assert!(app.is_ok(), "Multi-layer config test should succeed");

        // Note: In production, the FlatConfig would be stored and merged with
        // application-level and stream-level TOML configurations before being
        // used to initialize the actual source/sink extensions.
    }
}
