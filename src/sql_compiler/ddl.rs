// SPDX-License-Identifier: MIT OR Apache-2.0

//! DDL Parser - Parse CREATE STREAM Statements
//!
//! Parses Data Definition Language (DDL) statements like CREATE STREAM.

use sqlparser::ast::{ColumnDef, ObjectName, Statement};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

use crate::query_api::definition::attribute::Type as AttributeType;
use crate::query_api::definition::StreamDefinition;

use super::catalog::SqlCatalog;
use super::error::DdlError;
use super::type_mapping::sql_type_to_attribute_type;

/// Information extracted from CREATE STREAM statement
#[derive(Debug, Clone)]
pub struct CreateStreamInfo {
    pub name: String,
    pub columns: Vec<ColumnDef>,
}

/// DDL Parser for CREATE STREAM statements
pub struct DdlParser;

impl DdlParser {
    /// Check if a SQL statement is CREATE STREAM
    pub fn is_create_stream(sql: &str) -> bool {
        sql.trim().to_uppercase().starts_with("CREATE STREAM")
    }

    /// Parse CREATE STREAM statement
    pub fn parse_create_stream(sql: &str) -> Result<CreateStreamInfo, DdlError> {
        // Replace STREAM with TABLE so sqlparser-rs can parse it
        let normalized_sql = sql
            .replace("CREATE STREAM", "CREATE TABLE")
            .replace("create stream", "CREATE TABLE");

        // Parse using sqlparser-rs
        let statements = Parser::parse_sql(&GenericDialect, &normalized_sql)
            .map_err(|e| DdlError::SqlParseFailed(e.to_string()))?;

        if statements.is_empty() {
            return Err(DdlError::InvalidCreateStream(
                "No statements found".to_string(),
            ));
        }

        match &statements[0] {
            Statement::CreateTable(create_table) => {
                let stream_name = Self::extract_table_name(&create_table.name)?;

                Ok(CreateStreamInfo {
                    name: stream_name,
                    columns: create_table.columns.clone(),
                })
            }
            _ => Err(DdlError::InvalidCreateStream(
                "Expected CREATE TABLE statement".to_string(),
            )),
        }
    }

    /// Extract table/stream name from ObjectName
    fn extract_table_name(name: &ObjectName) -> Result<String, DdlError> {
        name.0
            .last()
            .and_then(|part| part.as_ident())
            .map(|ident| ident.value.clone())
            .ok_or_else(|| DdlError::InvalidCreateStream("No table name found".to_string()))
    }

    /// Convert CREATE STREAM info to StreamDefinition
    pub fn create_stream_definition(info: &CreateStreamInfo) -> Result<StreamDefinition, DdlError> {
        let mut stream_def = StreamDefinition::new(info.name.clone());

        for column in &info.columns {
            let attr_type = sql_type_to_attribute_type(&column.data_type)
                .map_err(|e| DdlError::InvalidCreateStream(e.to_string()))?;

            stream_def = stream_def.attribute(column.name.value.clone(), attr_type);
        }

        Ok(stream_def)
    }

    /// Parse and register CREATE STREAM in catalog (convenience method)
    pub fn register_create_stream(sql: &str, catalog: &mut SqlCatalog) -> Result<(), DdlError> {
        let info = Self::parse_create_stream(sql)?;
        let stream_def = Self::create_stream_definition(&info)?;
        catalog.register_stream(info.name.clone(), stream_def)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_create_stream() {
        assert!(DdlParser::is_create_stream("CREATE STREAM Foo (x INT)"));
        assert!(DdlParser::is_create_stream("create stream Foo (x INT)"));
        assert!(!DdlParser::is_create_stream("SELECT * FROM Foo"));
    }

    #[test]
    fn test_parse_simple_create_stream() {
        let sql = "CREATE STREAM StockStream (symbol VARCHAR, price DOUBLE)";
        let info = DdlParser::parse_create_stream(sql).unwrap();

        assert_eq!(info.name, "StockStream");
        assert_eq!(info.columns.len(), 2);
        assert_eq!(info.columns[0].name.value, "symbol");
        assert_eq!(info.columns[1].name.value, "price");
    }

    #[test]
    fn test_parse_create_stream_with_types() {
        let sql = r#"
            CREATE STREAM SensorStream (
                id INT,
                temperature FLOAT,
                timestamp BIGINT,
                active BOOLEAN
            )
        "#;

        let info = DdlParser::parse_create_stream(sql).unwrap();
        assert_eq!(info.name, "SensorStream");
        assert_eq!(info.columns.len(), 4);
    }

    #[test]
    fn test_create_stream_definition() {
        let sql = "CREATE STREAM TestStream (name VARCHAR, age INT)";
        let info = DdlParser::parse_create_stream(sql).unwrap();
        let stream_def = DdlParser::create_stream_definition(&info).unwrap();

        assert_eq!(stream_def.abstract_definition.get_id(), "TestStream");
        assert_eq!(stream_def.abstract_definition.get_attribute_list().len(), 2);
        assert_eq!(
            stream_def.abstract_definition.get_attribute_list()[0].get_name(),
            "name"
        );
        assert_eq!(
            stream_def.abstract_definition.get_attribute_list()[0].get_type(),
            &AttributeType::STRING
        );
        assert_eq!(
            stream_def.abstract_definition.get_attribute_list()[1].get_name(),
            "age"
        );
        assert_eq!(
            stream_def.abstract_definition.get_attribute_list()[1].get_type(),
            &AttributeType::INT
        );
    }

    #[test]
    fn test_register_create_stream() {
        let mut catalog = SqlCatalog::new();
        let sql = "CREATE STREAM TestStream (x INT, y DOUBLE)";

        DdlParser::register_create_stream(sql, &mut catalog).unwrap();

        assert!(catalog.get_stream("TestStream").is_ok());
        assert!(catalog.has_column("TestStream", "x"));
        assert!(catalog.has_column("TestStream", "y"));
    }

    #[test]
    fn test_duplicate_stream_registration() {
        let mut catalog = SqlCatalog::new();
        let sql = "CREATE STREAM TestStream (x INT)";

        DdlParser::register_create_stream(sql, &mut catalog).unwrap();
        let result = DdlParser::register_create_stream(sql, &mut catalog);

        assert!(result.is_err());
    }

    #[test]
    fn test_extract_table_name() {
        let sql = "CREATE STREAM MyStream (x INT)";
        let normalized = sql.replace("CREATE STREAM", "CREATE TABLE");
        let statements = Parser::parse_sql(&GenericDialect, &normalized).unwrap();

        if let Statement::CreateTable(create_table) = &statements[0] {
            let extracted = DdlParser::extract_table_name(&create_table.name).unwrap();
            assert_eq!(extracted, "MyStream");
        }
    }
}
