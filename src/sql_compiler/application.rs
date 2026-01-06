// SPDX-License-Identifier: MIT OR Apache-2.0

//! Application Parser - Parse Complete SQL Applications
//!
//! Parses multi-statement SQL applications with DDL and queries.

use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

use crate::query_api::definition::TriggerDefinition;
use sqlparser::ast::{CreateStreamTrigger, StreamTriggerTiming};

use super::catalog::{SqlApplication, SqlCatalog};
use super::converter::SqlConverter;
use super::error::ApplicationError;
use super::normalization::normalize_stream_syntax;
use super::type_inference::TypeInferenceEngine;
use super::type_mapping::sql_type_to_attribute_type;
use super::with_clause::{extract_with_options, validate_with_clause};

/// Convert a parsed EventFlux streaming trigger to a TriggerDefinition
fn convert_stream_trigger(
    trigger: &CreateStreamTrigger,
) -> Result<TriggerDefinition, ApplicationError> {
    let name = trigger.name.to_string();

    match &trigger.timing {
        StreamTriggerTiming::Start => {
            Ok(TriggerDefinition::new(name).at("start".to_string()))
        }
        StreamTriggerTiming::Every { interval_ms } => {
            // interval_ms is pre-computed at parse time using parse_streaming_time_duration_ms()
            Ok(TriggerDefinition::new(name).at_every(*interval_ms as i64))
        }
        StreamTriggerTiming::Cron(expr) => {
            Ok(TriggerDefinition::new(name).at(expr.clone()))
        }
    }
}

/// Validate expression types in a query using the type inference engine
fn validate_query_types(
    query: &crate::query_api::execution::Query,
    catalog: &SqlCatalog,
) -> Result<(), ApplicationError> {
    let type_engine = TypeInferenceEngine::new(catalog);
    type_engine
        .validate_query(query)
        .map_err(ApplicationError::Type)
}

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
                let name = create.name.to_string();

                // Extract and validate WITH clause options
                let with_config = extract_with_options(&create.table_options)?;

                // Distinguish between TABLE and STREAM:
                // - STREAM: has 'type' property (source/sink/internal)
                // - TABLE: no 'type' but has 'extension' (e.g., cache/jdbc)
                // - STREAM: no 'type' and no 'extension' (pure internal stream)
                let is_table =
                    with_config.get("type").is_none() && with_config.get("extension").is_some();

                if is_table {
                    // This is a TABLE (e.g., CREATE TABLE T (...) WITH ('extension' = 'cache'))
                    let mut table_def =
                        crate::query_api::definition::TableDefinition::new(name.clone());

                    // Extract column definitions
                    for col in &create.columns {
                        let attr_type = sql_type_to_attribute_type(&col.data_type)?;
                        table_def = table_def.attribute(col.name.value.clone(), attr_type);
                    }

                    if !with_config.is_empty() {
                        validate_with_clause(&with_config)?;
                        table_def = table_def.with_config(with_config);
                    }

                    catalog.register_table(name, table_def);
                } else {
                    // This is a STREAM (e.g., CREATE STREAM S (...) or CREATE STREAM S (...) WITH ('type' = 'source'))
                    let mut stream_def =
                        crate::query_api::definition::StreamDefinition::new(name.clone());

                    // Extract column definitions
                    for col in &create.columns {
                        let attr_type = sql_type_to_attribute_type(&col.data_type)?;
                        stream_def = stream_def.attribute(col.name.value.clone(), attr_type);
                    }

                    if !with_config.is_empty() {
                        validate_with_clause(&with_config)?;
                        stream_def = stream_def.with_config(with_config);
                    }

                    catalog.register_stream(name, stream_def)?;
                }
            }
            sqlparser::ast::Statement::Query(query) => {
                // Convert query AST directly (no re-parsing!)
                let q = SqlConverter::convert_query_ast(&query, &catalog, None)?;

                // Type validation: validate expression types in the query
                validate_query_types(&q, &catalog)?;

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

                // Type validation: validate expression types in the query
                validate_query_types(&q, &catalog)?;

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
            sqlparser::ast::Statement::CreateStreamTrigger(stream_trigger) => {
                // Convert EventFlux streaming trigger to TriggerDefinition
                let trigger_def = convert_stream_trigger(&stream_trigger)?;
                catalog.register_trigger(trigger_def);
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
    fn test_create_stream_without_format_allowed() {
        // Format is now optional at SQL level (some sources like timer use binary passthrough)
        let sql = r#"
            CREATE STREAM TimerStream (tick STRING)
            WITH (
                type = 'source',
                extension = 'timer'
            );
        "#;

        let result = parse_sql_application(sql);
        assert!(
            result.is_ok(),
            "Should allow missing format for sources like timer"
        );

        // Note: Sources that require format (like Kafka) will fail at initialization time,
        // not at SQL parsing time. This allows binary passthrough sources (timer) to work.
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

    // ========================================================================
    // WITH Config Storage Tests - Verify end-to-end flow
    // ========================================================================

    #[test]
    fn test_with_config_stored_in_stream_definition() {
        let sql = r#"
            CREATE STREAM TimerInput (tick BIGINT)
            WITH (
                type = 'source',
                extension = 'timer',
                "timer.interval" = '5000',
                format = 'json'
            );
        "#;

        let app = parse_sql_application(sql).expect("Failed to parse SQL with WITH clause");

        // Verify stream was registered
        let stream_def = app
            .catalog
            .get_stream("TimerInput")
            .expect("Stream not found in catalog");

        // Verify WITH config was stored
        assert!(
            stream_def.with_config.is_some(),
            "WITH config should be stored in StreamDefinition"
        );

        let config = stream_def.with_config.as_ref().unwrap();

        // Verify all properties were captured
        assert_eq!(
            config.get("type"),
            Some(&"source".to_string()),
            "type property should be stored"
        );
        assert_eq!(
            config.get("extension"),
            Some(&"timer".to_string()),
            "extension property should be stored"
        );
        assert_eq!(
            config.get("timer.interval"),
            Some(&"5000".to_string()),
            "timer.interval property should be stored"
        );
        assert_eq!(
            config.get("format"),
            Some(&"json".to_string()),
            "format property should be stored"
        );
    }

    #[test]
    fn test_stream_without_with_clause_has_no_config() {
        let sql = r#"
            CREATE STREAM SimpleStream (id INT, value DOUBLE);
        "#;

        let app = parse_sql_application(sql).expect("Failed to parse stream without WITH");

        let stream_def = app
            .catalog
            .get_stream("SimpleStream")
            .expect("Stream not found");

        // Streams without WITH clause should have None for with_config
        assert!(
            stream_def.with_config.is_none(),
            "Stream without WITH clause should have no stored config"
        );
    }

    #[test]
    fn test_with_config_custom_properties_stored() {
        let sql = r#"
            CREATE STREAM KafkaInput (message VARCHAR)
            WITH (
                type = 'source',
                extension = 'kafka',
                format = 'json',
                "kafka.bootstrap.servers" = 'localhost:9092',
                "kafka.topic" = 'events',
                "kafka.consumer.group" = 'my-group',
                "json.date-format" = 'yyyy-MM-dd HH:mm:ss'
            );
        "#;

        let app = parse_sql_application(sql).expect("Failed to parse Kafka source with properties");

        let stream_def = app
            .catalog
            .get_stream("KafkaInput")
            .expect("Stream not found");

        assert!(stream_def.with_config.is_some());
        let config = stream_def.with_config.as_ref().unwrap();

        // Verify all custom properties stored
        assert_eq!(
            config.get("kafka.bootstrap.servers"),
            Some(&"localhost:9092".to_string())
        );
        assert_eq!(config.get("kafka.topic"), Some(&"events".to_string()));
        assert_eq!(
            config.get("kafka.consumer.group"),
            Some(&"my-group".to_string())
        );
        assert_eq!(
            config.get("json.date-format"),
            Some(&"yyyy-MM-dd HH:mm:ss".to_string())
        );
    }

    #[test]
    fn test_with_config_multiple_streams_independent() {
        let sql = r#"
            CREATE STREAM TimerSource (tick BIGINT)
            WITH (
                type = 'source',
                extension = 'timer',
                "timer.interval" = '1000',
                format = 'json'
            );

            CREATE STREAM LogSink (tick BIGINT)
            WITH (
                type = 'sink',
                extension = 'log',
                format = 'text',
                "log.prefix" = '[EVENT]'
            );
        "#;

        let app = parse_sql_application(sql).expect("Failed to parse multiple streams");

        let timer_def = app.catalog.get_stream("TimerSource").unwrap();
        let log_def = app.catalog.get_stream("LogSink").unwrap();

        // Both should have configs
        assert!(timer_def.with_config.is_some());
        assert!(log_def.with_config.is_some());

        let timer_config = timer_def.with_config.as_ref().unwrap();
        let log_config = log_def.with_config.as_ref().unwrap();

        // Verify independence - timer config shouldn't have log properties
        assert_eq!(timer_config.get("extension"), Some(&"timer".to_string()));
        assert_eq!(
            timer_config.get("timer.interval"),
            Some(&"1000".to_string())
        );
        assert!(timer_config.get("log.prefix").is_none());

        // Log config shouldn't have timer properties
        assert_eq!(log_config.get("extension"), Some(&"log".to_string()));
        assert_eq!(log_config.get("log.prefix"), Some(&"[EVENT]".to_string()));
        assert!(log_config.get("timer.interval").is_none());
    }

    #[test]
    fn test_with_config_empty_clause_treated_as_none() {
        let sql = r#"
            CREATE STREAM EmptyWith (x INT) WITH ();
        "#;

        let app = parse_sql_application(sql).expect("Failed to parse stream with empty WITH");

        let stream_def = app.catalog.get_stream("EmptyWith").unwrap();

        // Empty WITH clause should not store config
        assert!(
            stream_def.with_config.is_none(),
            "Empty WITH clause should not store configuration"
        );
    }
}
