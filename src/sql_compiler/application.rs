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
    let mut catalog = SqlCatalog::new();
    let mut queries = Vec::new();

    // Split SQL into individual statements
    let statements = split_sql_statements(sql);

    if statements.is_empty() {
        return Err(ApplicationError::EmptyApplication);
    }

    // Process each statement
    for stmt_text in statements {
        let trimmed = stmt_text.trim();

        if trimmed.is_empty() {
            continue;
        }

        // Check if it's a CREATE STREAM statement
        if DdlParser::is_create_stream(trimmed) {
            DdlParser::register_create_stream(trimmed, &mut catalog)?;
        } else {
            // Parse as query
            let query = SqlConverter::convert(trimmed, &catalog)?;
            queries.push(query);
        }
    }

    Ok(SqlApplication::new(catalog, queries))
}

/// Split SQL text into individual statements
fn split_sql_statements(sql: &str) -> Vec<String> {
    // Simple semicolon split (can be enhanced with proper parsing)
    sql.split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
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
        assert_eq!(app.queries.len(), 1);
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
        assert_eq!(app.queries.len(), 2);
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
        assert_eq!(app.queries.len(), 1);
    }

    #[test]
    fn test_empty_application_error() {
        let sql = "";
        let result = parse_sql_application(sql);
        assert!(result.is_err());
    }

    #[test]
    fn test_split_statements() {
        let sql = "CREATE STREAM S1 (x INT); SELECT x FROM S1; SELECT x FROM S1";
        let stmts = split_sql_statements(sql);
        assert_eq!(stmts.len(), 3);
        assert!(stmts[0].contains("CREATE STREAM"));
        assert!(stmts[1].contains("SELECT"));
        assert!(stmts[2].contains("SELECT"));
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
        assert_eq!(app.queries.len(), 1);
    }
}
