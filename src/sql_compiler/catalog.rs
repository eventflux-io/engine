// SPDX-License-Identifier: MIT OR Apache-2.0

//! SQL Catalog - Schema Management System
//!
//! Provides schema management for SQL queries, tracking stream and table definitions.

use crate::query_api::definition::abstract_definition::AbstractDefinition;
use crate::query_api::definition::attribute::{Attribute, Type as AttributeType};
use crate::query_api::definition::{StreamDefinition, TableDefinition};
use crate::query_api::eventflux_app::EventFluxApp;
use crate::query_api::execution::ExecutionElement;
use crate::query_api::expression::Expression;
use sqlparser::ast::ColumnDef;
use std::collections::HashMap;
use std::sync::Arc;

use super::error::CatalogError;

/// A relation that can appear in SQL queries (either a stream or a table)
///
/// In EventFlux SQL, both streams and tables can appear in FROM and JOIN clauses.
/// This enum provides a unified type for schema lookups and validation.
///
/// # Semantics
///
/// - **Stream**: Temporal data source with time-ordered events
/// - **Table**: Stateful lookup table (cache or database-backed)
///
/// # Query API vs Runtime
///
/// The Query API layer uses `SingleInputStream` to reference both streams and tables
/// by name (late binding). The runtime layer determines the actual type by checking
/// `EventFluxApp.table_definition_map` and creates the appropriate processor:
///
/// - Stream-Stream JOIN → `JoinProcessor` (temporal windowed join)
/// - Stream-Table JOIN → `TableJoinProcessor` (enrichment lookup)
///
/// This separation allows:
/// - Schema-independent query serialization
/// - Runtime optimization based on data statistics
/// - Flexible execution strategies
///
/// # Examples
///
/// ```sql
/// -- Stream-Stream JOIN (temporal)
/// SELECT * FROM stream1 JOIN stream2 ON stream1.id = stream2.id;
///
/// -- Stream-Table JOIN (enrichment)
/// SELECT * FROM events JOIN user_profiles ON events.userId = user_profiles.id;
///
/// -- FROM clause with table
/// SELECT * FROM customer_cache WHERE region = 'US';
/// ```
#[derive(Debug, Clone)]
pub enum Relation {
    /// A stream definition (temporal event source)
    Stream(Arc<StreamDefinition>),

    /// A table definition (stateful lookup table)
    Table(Arc<TableDefinition>),
}

impl Relation {
    /// Get the abstract definition (schema) from this relation
    ///
    /// Returns the schema information (attributes, types) regardless of whether
    /// this is a stream or table. Both share the `AbstractDefinition` base type.
    pub fn abstract_definition(&self) -> &AbstractDefinition {
        match self {
            Relation::Stream(stream) => &stream.abstract_definition,
            Relation::Table(table) => &table.abstract_definition,
        }
    }

    /// Check if this relation is a stream
    pub fn is_stream(&self) -> bool {
        matches!(self, Relation::Stream(_))
    }

    /// Check if this relation is a table
    pub fn is_table(&self) -> bool {
        matches!(self, Relation::Table(_))
    }
}

/// Information extracted from CREATE STREAM statement
#[derive(Debug, Clone)]
pub struct CreateStreamInfo {
    pub name: String,
    pub columns: Vec<ColumnDef>,
}

/// SQL Catalog manages stream and table schemas
#[derive(Debug, Clone)]
pub struct SqlCatalog {
    streams: HashMap<String, Arc<StreamDefinition>>,
    tables: HashMap<String, Arc<TableDefinition>>,
    aliases: HashMap<String, String>,
}

impl SqlCatalog {
    /// Create a new empty catalog
    pub fn new() -> Self {
        SqlCatalog {
            streams: HashMap::new(),
            tables: HashMap::new(),
            aliases: HashMap::new(),
        }
    }

    /// Register a stream definition
    pub fn register_stream(
        &mut self,
        name: String,
        definition: StreamDefinition,
    ) -> Result<(), CatalogError> {
        if self.streams.contains_key(&name) {
            return Err(CatalogError::DuplicateStream(name));
        }
        self.streams.insert(name, Arc::new(definition));
        Ok(())
    }

    /// Register a table definition
    pub fn register_table(&mut self, name: String, definition: TableDefinition) {
        self.tables.insert(name, Arc::new(definition));
    }

    /// Register an alias for a stream
    pub fn register_alias(&mut self, alias: String, stream_name: String) {
        self.aliases.insert(alias, stream_name);
    }

    /// Get a stream definition by name (or alias)
    pub fn get_stream(&self, name: &str) -> Result<Arc<StreamDefinition>, CatalogError> {
        // Try direct lookup
        if let Some(def) = self.streams.get(name) {
            return Ok(Arc::clone(def));
        }

        // Try alias lookup
        if let Some(actual_name) = self.aliases.get(name) {
            if let Some(def) = self.streams.get(actual_name) {
                return Ok(Arc::clone(def));
            }
        }

        Err(CatalogError::UnknownStream(name.to_string()))
    }

    /// Get a table definition by name
    pub fn get_table(&self, name: &str) -> Option<Arc<TableDefinition>> {
        self.tables.get(name).map(Arc::clone)
    }

    /// Get a relation (stream or table) by name for use in FROM/JOIN clauses
    ///
    /// This is the unified lookup method for any relation that can appear in SQL queries.
    /// Use this instead of `get_stream()` or `get_table()` when you need to handle both.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the stream or table to look up
    ///
    /// # Returns
    ///
    /// * `Ok(Relation)` - The relation (either Stream or Table variant)
    /// * `Err(CatalogError::UnknownRelation)` - If neither a stream nor table exists with that name
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Validate a relation exists in a FROM clause
    /// let relation = catalog.get_relation("user_events")?;
    ///
    /// // Check if it's a stream or table
    /// match relation {
    ///     Relation::Stream(s) => println!("Found stream: {}", s.abstract_definition.id),
    ///     Relation::Table(t) => println!("Found table: {}", t.abstract_definition.id),
    /// }
    ///
    /// // Access schema regardless of type
    /// let schema = relation.abstract_definition();
    /// ```
    pub fn get_relation(&self, name: &str) -> Result<Relation, CatalogError> {
        // Try stream first (more common case)
        if let Ok(stream) = self.get_stream(name) {
            return Ok(Relation::Stream(stream));
        }

        // Try table
        if let Some(table) = self.get_table(name) {
            return Ok(Relation::Table(table));
        }

        Err(CatalogError::UnknownRelation(name.to_string()))
    }

    /// Check if a relation (stream or table) exists
    ///
    /// Fast existence check without retrieving the full definition.
    /// Useful for validation without needing the schema information.
    pub fn has_relation(&self, name: &str) -> bool {
        self.get_stream(name).is_ok() || self.get_table(name).is_some()
    }

    /// Check if a column exists in a relation (stream or table)
    ///
    /// Uses unified relation lookup to check both streams and tables.
    /// Commonly used for validating column references in WHERE/ON clauses.
    ///
    /// # Arguments
    ///
    /// * `relation_name` - The name of the stream or table
    /// * `column_name` - The name of the column/attribute to check
    ///
    /// # Returns
    ///
    /// * `true` if the column exists in the relation
    /// * `false` if the relation doesn't exist or doesn't have that column
    pub fn has_column(&self, relation_name: &str, column_name: &str) -> bool {
        // Use unified relation lookup
        if let Ok(relation) = self.get_relation(relation_name) {
            return relation
                .abstract_definition()
                .get_attribute_list()
                .iter()
                .any(|attr| attr.get_name() == column_name);
        }

        false
    }

    /// Get column type from a stream
    pub fn get_column_type(
        &self,
        stream_name: &str,
        column_name: &str,
    ) -> Result<AttributeType, CatalogError> {
        let stream = self.get_stream(stream_name)?;

        stream
            .abstract_definition
            .get_attribute_list()
            .iter()
            .find(|attr| attr.get_name() == column_name)
            .map(|attr| *attr.get_type())
            .ok_or_else(|| {
                CatalogError::UnknownColumn(stream_name.to_string(), column_name.to_string())
            })
    }

    /// Get all columns from a stream
    pub fn get_all_columns(&self, stream_name: &str) -> Result<Vec<Attribute>, CatalogError> {
        let stream = self.get_stream(stream_name)?;
        Ok(stream.abstract_definition.get_attribute_list().to_vec())
    }

    /// Get all stream names
    pub fn get_stream_names(&self) -> Vec<String> {
        self.streams.keys().cloned().collect()
    }

    /// Check if catalog is empty
    pub fn is_empty(&self) -> bool {
        self.streams.is_empty() && self.tables.is_empty()
    }
}

impl Default for SqlCatalog {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a complete SQL application with catalog and execution elements
#[derive(Debug, Clone)]
pub struct SqlApplication {
    pub catalog: SqlCatalog,
    pub execution_elements: Vec<ExecutionElement>,
}

impl SqlApplication {
    /// Create a new SQL application
    pub fn new(catalog: SqlCatalog, execution_elements: Vec<ExecutionElement>) -> Self {
        SqlApplication {
            catalog,
            execution_elements,
        }
    }

    /// Get the catalog
    pub fn get_catalog(&self) -> &SqlCatalog {
        &self.catalog
    }

    /// Get the execution elements
    pub fn get_execution_elements(&self) -> &[ExecutionElement] {
        &self.execution_elements
    }

    /// Check if application is empty
    pub fn is_empty(&self) -> bool {
        self.execution_elements.is_empty()
    }

    /// Convert to EventFluxApp for runtime creation
    pub fn to_eventflux_app(self, app_name: String) -> EventFluxApp {
        let mut app = EventFluxApp::new(app_name);

        // Add all stream definitions from catalog
        for (stream_name, stream_def) in self.catalog.streams {
            app.stream_definition_map.insert(stream_name, stream_def);
        }

        // Add all table definitions from catalog
        for (table_name, table_def) in self.catalog.tables {
            app.table_definition_map.insert(table_name, table_def);
        }

        // Auto-create output streams from queries in execution elements
        for elem in &self.execution_elements {
            match elem {
                ExecutionElement::Query(query) => {
                    // Extract target stream name from query's output stream
                    let output_stream = query.get_output_stream();
                    let target_stream_name = output_stream
                        .get_target_id()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "OutputStream".to_string());

                    // Create output stream if it doesn't exist AND it's not a table
                    // CRITICAL FIX: Don't create StreamDefinition for tables!
                    if !app.table_definition_map.contains_key(&target_stream_name) {
                        app.stream_definition_map
                            .entry(target_stream_name.clone())
                            .or_insert_with(|| {
                                let selector = query.get_selector();
                                let mut output_stream =
                                    StreamDefinition::new(target_stream_name.clone());

                                // Add attributes from selector output
                                for output_attr in selector.get_selection_list() {
                                    let attr_name =
                                        output_attr.get_rename().clone().unwrap_or_else(|| {
                                            if let Expression::Variable(var) =
                                                output_attr.get_expression()
                                            {
                                                var.get_attribute_name().to_string()
                                            } else {
                                                "output".to_string()
                                            }
                                        });

                                    // Default to STRING type (type inference would be better)
                                    output_stream =
                                        output_stream.attribute(attr_name, AttributeType::STRING);
                                }

                                Arc::new(output_stream)
                            });
                    }
                }
                ExecutionElement::Partition(partition) => {
                    // Auto-create output streams from queries within partition
                    for query in &partition.query_list {
                        let output_stream = query.get_output_stream();
                        let target_stream_name = output_stream
                            .get_target_id()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| "OutputStream".to_string());

                        // Don't create StreamDefinition for tables
                        if !app.table_definition_map.contains_key(&target_stream_name) {
                            app.stream_definition_map
                                .entry(target_stream_name.clone())
                                .or_insert_with(|| {
                                    let selector = query.get_selector();
                                    let mut output_stream =
                                        StreamDefinition::new(target_stream_name.clone());

                                    for output_attr in selector.get_selection_list() {
                                        let attr_name =
                                            output_attr.get_rename().clone().unwrap_or_else(|| {
                                                if let Expression::Variable(var) =
                                                    output_attr.get_expression()
                                                {
                                                    var.get_attribute_name().to_string()
                                                } else {
                                                    "output".to_string()
                                                }
                                            });

                                        output_stream =
                                            output_stream.attribute(attr_name, AttributeType::STRING);
                                    }

                                    Arc::new(output_stream)
                                });
                        }
                    }
                }
            }
        }

        // Add all execution elements
        for elem in self.execution_elements {
            app.add_execution_element(elem);
        }

        app
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_creation() {
        let catalog = SqlCatalog::new();
        assert!(catalog.is_empty());
    }

    #[test]
    fn test_register_stream() {
        let mut catalog = SqlCatalog::new();
        let stream = StreamDefinition::new("TestStream".to_string())
            .attribute("col1".to_string(), AttributeType::STRING);

        catalog
            .register_stream("TestStream".to_string(), stream)
            .unwrap();
        assert!(!catalog.is_empty());
        assert!(catalog.get_stream("TestStream").is_ok());
    }

    #[test]
    fn test_duplicate_stream_error() {
        let mut catalog = SqlCatalog::new();
        let stream1 = StreamDefinition::new("TestStream".to_string());
        let stream2 = StreamDefinition::new("TestStream".to_string());

        catalog
            .register_stream("TestStream".to_string(), stream1)
            .unwrap();
        let result = catalog.register_stream("TestStream".to_string(), stream2);
        assert!(result.is_err());
    }

    #[test]
    fn test_has_column() {
        let mut catalog = SqlCatalog::new();
        let stream = StreamDefinition::new("TestStream".to_string())
            .attribute("col1".to_string(), AttributeType::STRING)
            .attribute("col2".to_string(), AttributeType::INT);

        catalog
            .register_stream("TestStream".to_string(), stream)
            .unwrap();
        assert!(catalog.has_column("TestStream", "col1"));
        assert!(catalog.has_column("TestStream", "col2"));
        assert!(!catalog.has_column("TestStream", "col3"));
    }

    #[test]
    fn test_get_column_type() {
        let mut catalog = SqlCatalog::new();
        let stream = StreamDefinition::new("TestStream".to_string())
            .attribute("name".to_string(), AttributeType::STRING)
            .attribute("age".to_string(), AttributeType::INT);

        catalog
            .register_stream("TestStream".to_string(), stream)
            .unwrap();

        let name_type = catalog.get_column_type("TestStream", "name").unwrap();
        assert_eq!(name_type, AttributeType::STRING);

        let age_type = catalog.get_column_type("TestStream", "age").unwrap();
        assert_eq!(age_type, AttributeType::INT);
    }

    #[test]
    fn test_get_all_columns() {
        let mut catalog = SqlCatalog::new();
        let stream = StreamDefinition::new("TestStream".to_string())
            .attribute("col1".to_string(), AttributeType::STRING)
            .attribute("col2".to_string(), AttributeType::INT);

        catalog
            .register_stream("TestStream".to_string(), stream)
            .unwrap();
        let columns = catalog.get_all_columns("TestStream").unwrap();
        assert_eq!(columns.len(), 2);
        assert_eq!(columns[0].get_name(), "col1");
        assert_eq!(columns[1].get_name(), "col2");
    }

    #[test]
    fn test_alias() {
        let mut catalog = SqlCatalog::new();
        let stream = StreamDefinition::new("TestStream".to_string())
            .attribute("col1".to_string(), AttributeType::STRING);

        catalog
            .register_stream("TestStream".to_string(), stream)
            .unwrap();
        catalog.register_alias("ts".to_string(), "TestStream".to_string());

        assert!(catalog.get_stream("ts").is_ok());
        assert!(catalog.has_column("ts", "col1"));
    }
}
