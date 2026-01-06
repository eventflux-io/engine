// SPDX-License-Identifier: MIT OR Apache-2.0

//! Error types for SQL compiler

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SqlCompilerError {
    #[error("Catalog error: {0}")]
    Catalog(#[from] CatalogError),

    #[error("Type error: {0}")]
    Type(#[from] TypeError),

    #[error("Expansion error: {0}")]
    Expansion(#[from] ExpansionError),

    #[error("Converter error: {0}")]
    Converter(#[from] ConverterError),

    #[error("Application error: {0}")]
    Application(#[from] ApplicationError),
}

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("Duplicate stream definition: {0}")]
    DuplicateStream(String),

    #[error("Unknown stream: {0}")]
    UnknownStream(String),

    #[error("Unknown relation (stream or table): {0}")]
    UnknownRelation(String),

    #[error("Unknown column: {0}.{1}")]
    UnknownColumn(String, String),
}

#[derive(Debug, Error)]
pub enum TypeError {
    #[error("Unsupported SQL type: {0}")]
    UnsupportedType(String),

    #[error("Type conversion failed: {0}")]
    ConversionFailed(String),

    #[error("Precision loss warning for {0}")]
    PrecisionLoss(String),
}

#[derive(Debug, Error)]
pub enum ExpansionError {
    #[error("Unknown stream: {0}")]
    UnknownStream(String),

    #[error("Unknown column: {0}.{1}")]
    UnknownColumn(String, String),

    #[error("Ambiguous column reference: {0}")]
    AmbiguousColumn(String),

    #[error("Invalid SELECT item: {0}")]
    InvalidSelectItem(String),

    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),
}

#[derive(Debug, Error)]
pub enum ConverterError {
    #[error("Unsupported SQL feature: {0}")]
    UnsupportedFeature(String),

    #[error("Invalid expression: {0}")]
    InvalidExpression(String),

    #[error("Schema not found for relation (stream or table): {0}")]
    SchemaNotFound(String),

    #[error("Conversion failed: {0}")]
    ConversionFailed(String),

    #[error("Direct table query not allowed: '{0}'. Tables must be used with JOIN. Example: FROM Stream JOIN {0} ON Stream.id = {0}.id")]
    DirectTableQuery(String),

    #[error("Table '{0}' cannot be used in pattern/sequence. Only streams can be used in PATTERN or SEQUENCE clauses.")]
    TableInPattern(String),

    #[error("Aggregation '{0}' cannot be queried directly. Use: FROM Stream JOIN {0} ON Stream.timestamp WITHIN ...")]
    DirectAggregationQuery(String),

    #[error("Aggregation '{0}' cannot be used in pattern/sequence. Only streams can be used in PATTERN or SEQUENCE clauses.")]
    AggregationInPattern(String),
}

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("Empty SQL application")]
    EmptyApplication,

    #[error("Catalog error: {0}")]
    Catalog(#[from] CatalogError),

    #[error("Converter error: {0}")]
    Converter(#[from] ConverterError),

    #[error("Type error: {0}")]
    Type(#[from] TypeError),
}
