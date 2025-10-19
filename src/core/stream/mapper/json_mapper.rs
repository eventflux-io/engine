// SPDX-License-Identifier: MIT OR Apache-2.0

//! JSON Mapper Implementation
//!
//! Provides bidirectional mapping between JSON and EventFlux events.
//!
//! ## Source Mapping (JSON → Events)
//!
//! ### Auto-Mapping (No `mapping.*` properties)
//! ```json
//! {"orderId": "123", "amount": 100.0}
//! ```
//! Maps all top-level fields by name.
//!
//! ### Explicit Mapping (With `mapping.*` properties)
//! ```toml
//! json.mapping.orderId = "$.order.id"
//! json.mapping.amount = "$.order.total"
//! ```
//! Extracts nested fields using JSONPath.
//!
//! ## Sink Mapping (Events → JSON)
//!
//! ### Simple Serialization (No template)
//! Converts events to JSON objects with field names from schema.
//!
//! ### Template-Based (With template)
//! ```toml
//! json.template = "{\"eventType\":\"ORDER\",\"id\":\"{{orderId}}\",\"amount\":{{amount}}}"
//! ```

use super::{SinkMapper, SourceMapper};
use crate::core::event::{AttributeValue, Event};
use crate::core::exception::EventFluxError;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Source mapper for JSON format
#[derive(Debug, Clone)]
pub struct JsonSourceMapper {
    /// Field name → JSONPath mappings
    /// Empty = auto-map all top-level fields
    mappings: HashMap<String, String>,
    /// Whether to ignore parse errors (continue processing)
    ignore_parse_errors: bool,
    /// Optional date format for parsing timestamps
    date_format: Option<String>,
    /// Maximum input size in bytes (default: 10 MB)
    max_input_size: usize,
    /// Maximum JSONPath nesting depth (default: 32)
    max_nesting_depth: usize,
}

impl JsonSourceMapper {
    /// Create a new JSON source mapper with default settings
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
            ignore_parse_errors: false,
            date_format: None,
            max_input_size: 10 * 1024 * 1024, // 10 MB default
            max_nesting_depth: 32,             // 32 levels default
        }
    }

    /// Create a new JSON source mapper with explicit mappings
    pub fn with_mappings(mappings: HashMap<String, String>) -> Self {
        Self {
            mappings,
            ignore_parse_errors: false,
            date_format: None,
            max_input_size: 10 * 1024 * 1024, // 10 MB default
            max_nesting_depth: 32,             // 32 levels default
        }
    }

    /// Set whether to ignore parse errors
    pub fn set_ignore_parse_errors(&mut self, ignore: bool) {
        self.ignore_parse_errors = ignore;
    }

    /// Set date format for timestamp parsing
    pub fn set_date_format(&mut self, format: Option<String>) {
        self.date_format = format;
    }

    /// Set maximum input size in bytes (for DoS protection)
    pub fn set_max_input_size(&mut self, max_size: usize) {
        self.max_input_size = max_size;
    }

    /// Set maximum JSONPath nesting depth (for DoS protection)
    pub fn set_max_nesting_depth(&mut self, max_depth: usize) {
        self.max_nesting_depth = max_depth;
    }

    /// Auto-map all top-level JSON fields to event attributes
    fn auto_map(&self, json: &JsonValue) -> Result<Vec<Event>, EventFluxError> {
        let obj = json
            .as_object()
            .ok_or_else(|| EventFluxError::MappingFailed {
                message: "JSON root must be an object for auto-mapping".to_string(),
                source: None,
            })?;

        // Sort keys for consistent ordering
        let mut sorted_keys: Vec<_> = obj.keys().collect();
        sorted_keys.sort();

        let mut event_data = Vec::new();
        for key in sorted_keys {
            if let Some(value) = obj.get(key) {
                event_data.push(json_value_to_attribute(value, self.date_format.as_deref())?);
            }
        }

        // Use current timestamp in milliseconds
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        Ok(vec![Event::new_with_data(timestamp, event_data)])
    }

    /// Extract fields using explicit JSONPath mappings
    fn extract_with_mappings(
        &self,
        json: &JsonValue,
        mappings: &HashMap<String, String>,
    ) -> Result<Vec<Event>, EventFluxError> {
        let mut event_data = Vec::new();

        // Extract fields in a consistent order (sorted by field name)
        let mut sorted_mappings: Vec<_> = mappings.iter().collect();
        sorted_mappings.sort_by_key(|(field_name, _)| *field_name);

        for (_field_name, json_path) in sorted_mappings {
            let value = extract_json_path(json, json_path, self.max_nesting_depth)?;
            event_data.push(json_value_to_attribute(&value, self.date_format.as_deref())?);
        }

        // Use current timestamp in milliseconds
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        Ok(vec![Event::new_with_data(timestamp, event_data)])
    }
}

impl Default for JsonSourceMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceMapper for JsonSourceMapper {
    fn map(&self, input: &[u8]) -> Result<Vec<Event>, EventFluxError> {
        // Check input size to prevent DoS attacks
        if input.len() > self.max_input_size {
            return Err(EventFluxError::MappingFailed {
                message: format!(
                    "Input size {} bytes exceeds maximum allowed {} bytes",
                    input.len(),
                    self.max_input_size
                ),
                source: None,
            });
        }

        // Parse JSON with error handling that respects ignore_parse_errors
        let json: JsonValue = match serde_json::from_slice(input) {
            Ok(v) => v,
            Err(e) => {
                if self.ignore_parse_errors {
                    // Skip this event - return empty list
                    return Ok(Vec::new());
                } else {
                    return Err(EventFluxError::MappingFailed {
                        message: format!("JSON parse error: {}", e),
                        source: Some(Box::new(e)),
                    });
                }
            }
        };

        // Apply all-or-nothing auto-mapping policy
        if self.mappings.is_empty() {
            self.auto_map(&json)
        } else {
            self.extract_with_mappings(&json, &self.mappings)
        }
    }

    fn clone_box(&self) -> Box<dyn SourceMapper> {
        Box::new(self.clone())
    }
}

/// Sink mapper for JSON format
#[derive(Debug, Clone)]
pub struct JsonSinkMapper {
    /// Optional template string for custom JSON output
    /// Uses {{fieldName}} placeholders
    template: Option<String>,
    /// Whether to pretty-print JSON (with indentation)
    pretty_print: bool,
}

impl JsonSinkMapper {
    /// Create a new JSON sink mapper without template (simple serialization)
    pub fn new() -> Self {
        Self {
            template: None,
            pretty_print: false,
        }
    }

    /// Create a new JSON sink mapper with a template
    pub fn with_template(template: String) -> Self {
        Self {
            template: Some(template),
            pretty_print: false,
        }
    }

    /// Enable pretty-printing (formatted JSON with indentation)
    pub fn set_pretty_print(&mut self, pretty: bool) {
        self.pretty_print = pretty;
    }
}

impl Default for JsonSinkMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl SinkMapper for JsonSinkMapper {
    fn map(&self, events: &[Event]) -> Result<Vec<u8>, EventFluxError> {
        if events.is_empty() {
            return Ok(Vec::new());
        }

        let event = &events[0]; // Process first event (batching can be added later)

        if let Some(template) = &self.template {
            // Use template rendering
            let rendered = render_template(template, event)?;
            Ok(rendered.into_bytes())
        } else {
            // Simple JSON serialization
            let json_value = event_to_json(event)?;
            let json_str = if self.pretty_print {
                serde_json::to_string_pretty(&json_value)
            } else {
                serde_json::to_string(&json_value)
            }
            .map_err(|e| EventFluxError::MappingFailed {
                message: format!("JSON serialization error: {}", e),
                source: Some(Box::new(e)),
            })?;
            Ok(json_str.into_bytes())
        }
    }

    fn clone_box(&self) -> Box<dyn SinkMapper> {
        Box::new(self.clone())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract value from JSON using JSONPath expression
///
/// **Supported JSONPath Syntax**:
/// - `$.field` - Top-level field
/// - `$.nested.field` - Nested field
/// - `$.array[0]` - Array element
/// - `$.nested.array[0].field` - Complex path
/// Extract value from JSON using JSONPath
///
/// # Parameters
/// - `json`: The JSON value to extract from
/// - `path`: The JSONPath expression (e.g., "$.field" or "$.nested.field")
/// - `max_depth`: Maximum allowed nesting depth (for DoS protection)
pub fn extract_json_path(
    json: &JsonValue,
    path: &str,
    max_depth: usize,
) -> Result<JsonValue, EventFluxError> {
    if !path.starts_with("$.") {
        return Err(EventFluxError::MappingFailed {
            message: format!("Invalid JSONPath '{}'. Must start with '$.'", path),
            source: None,
        });
    }

    let path_parts: Vec<&str> = path[2..].split('.').collect();

    // Check nesting depth to prevent DoS attacks
    if path_parts.len() > max_depth {
        return Err(EventFluxError::MappingFailed {
            message: format!(
                "JSONPath nesting depth {} exceeds maximum allowed {}",
                path_parts.len(),
                max_depth
            ),
            source: None,
        });
    }

    let mut current = json;

    for part in path_parts {
        // Handle array indexing: field[index]
        if let Some(bracket_idx) = part.find('[') {
            let field_name = &part[..bracket_idx];
            let index_str = &part[bracket_idx + 1..part.len() - 1];
            let index = index_str
                .parse::<usize>()
                .map_err(|_| EventFluxError::MappingFailed {
                    message: format!("Invalid array index in path: {}", part),
                    source: None,
                })?;

            current = current
                .get(field_name)
                .ok_or_else(|| EventFluxError::MappingFailed {
                    message: format!("Field '{}' not found in JSON", field_name),
                    source: None,
                })?;

            current = current
                .get(index)
                .ok_or_else(|| EventFluxError::MappingFailed {
                    message: format!("Array index {} out of bounds", index),
                    source: None,
                })?;
        } else {
            current = current
                .get(part)
                .ok_or_else(|| EventFluxError::MappingFailed {
                    message: format!("Field '{}' not found in JSON", part),
                    source: None,
                })?;
        }
    }

    Ok(current.clone())
}

/// Convert JSON value to AttributeValue
/// Convert JSON value to AttributeValue with optional date parsing
///
/// If `date_format` is provided, attempts to parse string values as dates using the format.
/// If parsing succeeds, returns the timestamp in milliseconds as AttributeValue::Long.
/// If parsing fails, keeps the value as AttributeValue::String.
pub fn json_value_to_attribute(
    value: &JsonValue,
    date_format: Option<&str>,
) -> Result<AttributeValue, EventFluxError> {
    match value {
        JsonValue::String(s) => {
            // Try to parse as date if date_format is configured
            if let Some(format) = date_format {
                if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, format) {
                    // Convert to timestamp in milliseconds
                    let timestamp = dt.and_utc().timestamp_millis();
                    return Ok(AttributeValue::Long(timestamp));
                }
                // If parsing fails, fall through to return as String
            }
            Ok(AttributeValue::String(s.clone()))
        }
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                // Check if it fits in i32
                if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                    Ok(AttributeValue::Int(i as i32))
                } else {
                    Ok(AttributeValue::Long(i))
                }
            } else if let Some(f) = n.as_f64() {
                Ok(AttributeValue::Double(f))
            } else {
                Err(EventFluxError::MappingFailed {
                    message: format!("Unsupported number format: {}", n),
                    source: None,
                })
            }
        }
        JsonValue::Bool(b) => Ok(AttributeValue::Bool(*b)),
        JsonValue::Null => Ok(AttributeValue::Null),
        JsonValue::Array(_) | JsonValue::Object(_) => {
            // For complex types, serialize to JSON string
            let json_str =
                serde_json::to_string(value).map_err(|e| EventFluxError::MappingFailed {
                    message: format!("Failed to serialize complex JSON value: {}", e),
                    source: Some(Box::new(e)),
                })?;
            Ok(AttributeValue::String(json_str))
        }
    }
}

/// Convert Event to JSON value
pub fn event_to_json(event: &Event) -> Result<JsonValue, EventFluxError> {
    let mut obj = serde_json::Map::new();

    // Add timestamp
    obj.insert(
        "_timestamp".to_string(),
        JsonValue::Number(event.timestamp.into()),
    );

    // Add event data
    for (idx, attr_value) in event.data.iter().enumerate() {
        let field_name = format!("field_{}", idx); // Generic field names (schema would provide real names)
        let json_value = attribute_to_json_value(attr_value)?;
        obj.insert(field_name, json_value);
    }

    Ok(JsonValue::Object(obj))
}

/// Convert AttributeValue to JSON value
pub fn attribute_to_json_value(attr: &AttributeValue) -> Result<JsonValue, EventFluxError> {
    match attr {
        AttributeValue::String(s) => Ok(JsonValue::String(s.clone())),
        AttributeValue::Int(i) => Ok(JsonValue::Number((*i).into())),
        AttributeValue::Long(l) => Ok(JsonValue::Number((*l).into())),
        AttributeValue::Float(f) => serde_json::Number::from_f64(*f as f64)
            .map(JsonValue::Number)
            .ok_or_else(|| EventFluxError::MappingFailed {
                message: format!("Invalid float value: {}", f),
                source: None,
            }),
        AttributeValue::Double(d) => serde_json::Number::from_f64(*d)
            .map(JsonValue::Number)
            .ok_or_else(|| EventFluxError::MappingFailed {
                message: format!("Invalid double value: {}", d),
                source: None,
            }),
        AttributeValue::Bool(b) => Ok(JsonValue::Bool(*b)),
        AttributeValue::Null => Ok(JsonValue::Null),
        AttributeValue::Object(_) => Ok(JsonValue::String("<object>".to_string())),
    }
}

/// Render template with event data
///
/// **Template Variables**:
/// - `{{fieldName}}` - Any stream attribute (by index field_0, field_1, etc.)
/// - `{{_timestamp}}` - Event processing timestamp
/// - `{{_eventTime}}` - Original event time (if available)
/// - `{{_streamName}}` - Source stream name (if available)
///
/// **Implementation**: Simple text replacement (no complex logic)
pub fn render_template(template: &str, event: &Event) -> Result<String, EventFluxError> {
    let mut result = template.to_string();

    // Replace system variables
    result = result.replace("{{_timestamp}}", &event.timestamp.to_string());

    // Replace event attributes (field_0, field_1, etc.)
    for (idx, attr_value) in event.data.iter().enumerate() {
        let field_name = format!("field_{}", idx);
        let placeholder = format!("{{{{{}}}}}", field_name);
        let value_str = attribute_value_to_string(attr_value);
        result = result.replace(&placeholder, &value_str);
    }

    Ok(result)
}

/// Convert AttributeValue to string for template rendering
pub fn attribute_value_to_string(value: &AttributeValue) -> String {
    match value {
        AttributeValue::String(s) => s.clone(),
        AttributeValue::Int(i) => i.to_string(),
        AttributeValue::Long(l) => l.to_string(),
        AttributeValue::Double(d) => d.to_string(),
        AttributeValue::Float(f) => f.to_string(),
        AttributeValue::Bool(b) => b.to_string(),
        AttributeValue::Null => "null".to_string(),
        AttributeValue::Object(_) => "<object>".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_mapping() {
        let json_str = r#"{"orderId": "123", "amount": 100.0}"#;
        let mapper = JsonSourceMapper::new();

        let events = mapper.map(json_str.as_bytes()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data.len(), 2);
    }

    #[test]
    fn test_explicit_mapping() {
        let json_str = r#"{"order": {"id": "123", "total": 100.0}}"#;
        let mut mappings = HashMap::new();
        mappings.insert("orderId".to_string(), "$.order.id".to_string());
        mappings.insert("amount".to_string(), "$.order.total".to_string());

        let mapper = JsonSourceMapper::with_mappings(mappings);

        let events = mapper.map(json_str.as_bytes()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data.len(), 2);
    }

    #[test]
    fn test_json_path_extraction() {
        let json = serde_json::json!({
            "order": {
                "id": "123",
                "items": [
                    {"name": "item1", "price": 10.0},
                    {"name": "item2", "price": 20.0}
                ]
            }
        });

        let value = extract_json_path(&json, "$.order.id", 32).unwrap();
        assert_eq!(value, serde_json::json!("123"));

        let value = extract_json_path(&json, "$.order.items[0].name", 32).unwrap();
        assert_eq!(value, serde_json::json!("item1"));
    }

    #[test]
    fn test_template_rendering() {
        let template = r#"{"eventType":"ORDER","id":"{{field_0}}","amount":{{field_1}}}"#;
        let event = Event::new_with_data(
            123,
            vec![
                AttributeValue::String("order-1".to_string()),
                AttributeValue::Double(100.0),
            ],
        );

        let rendered = render_template(template, &event).unwrap();
        assert!(rendered.contains("\"id\":\"order-1\""));
        assert!(rendered.contains("\"amount\":100"));
    }

    #[test]
    fn test_all_or_nothing_mapping_policy() {
        // All auto-mapped (no mappings)
        let mapper1 = JsonSourceMapper::new();
        assert!(mapper1.mappings.is_empty());

        // All explicit (any mapping present = all explicit)
        let mut mappings = HashMap::new();
        mappings.insert("field1".to_string(), "$.field1".to_string());
        let mapper2 = JsonSourceMapper::with_mappings(mappings);
        assert!(!mapper2.mappings.is_empty());
    }

    #[test]
    fn test_json_sink_simple() {
        let event = Event::new_with_data(
            123,
            vec![
                AttributeValue::String("test".to_string()),
                AttributeValue::Int(42),
            ],
        );

        let mapper = JsonSinkMapper::new();
        let result = mapper.map(&[event]).unwrap();
        let json_str = String::from_utf8(result).unwrap();

        // Verify it's valid JSON
        let _: JsonValue = serde_json::from_str(&json_str).unwrap();
    }

    #[test]
    fn test_json_sink_template() {
        let event = Event::new_with_data(
            123,
            vec![
                AttributeValue::String("test-id".to_string()),
                AttributeValue::Double(99.5),
            ],
        );

        let template = r#"{"id":"{{field_0}}","value":{{field_1}}}"#.to_string();
        let mapper = JsonSinkMapper::with_template(template);
        let result = mapper.map(&[event]).unwrap();
        let json_str = String::from_utf8(result).unwrap();

        assert!(json_str.contains("\"id\":\"test-id\""));
        assert!(json_str.contains("\"value\":99.5"));
    }

    // ERROR CASE TESTS - Testing bug fixes

    #[test]
    fn test_ignore_parse_errors_enabled() {
        let mut mapper = JsonSourceMapper::new();
        mapper.set_ignore_parse_errors(true);

        // Malformed JSON should be skipped, not error
        let result = mapper.map(b"invalid json{{{");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0); // No events, just skipped
    }

    #[test]
    fn test_ignore_parse_errors_disabled() {
        let mapper = JsonSourceMapper::new(); // ignore_parse_errors = false by default

        // Malformed JSON should cause error
        let result = mapper.map(b"invalid json{{{");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("JSON parse error"));
    }

    #[test]
    fn test_date_format_parsing() {
        let mut mapper = JsonSourceMapper::new();
        mapper.set_date_format(Some("%Y-%m-%d %H:%M:%S".to_string()));

        let json_str = r#"{"timestamp": "2025-10-19 12:34:56", "value": 100}"#;
        let events = mapper.map(json_str.as_bytes()).unwrap();

        assert_eq!(events.len(), 1);
        // First field (timestamp) should be parsed as Long (milliseconds)
        assert!(matches!(events[0].data[0], AttributeValue::Long(_)));
        // Second field (value) should remain as Int
        assert!(matches!(events[0].data[1], AttributeValue::Int(100)));
    }

    #[test]
    fn test_date_format_parsing_fallback() {
        let mut mapper = JsonSourceMapper::new();
        mapper.set_date_format(Some("%Y-%m-%d".to_string()));

        // Date doesn't match format, should fall back to String
        let json_str = r#"{"timestamp": "not a date", "value": 100}"#;
        let events = mapper.map(json_str.as_bytes()).unwrap();

        assert_eq!(events.len(), 1);
        // Should keep as String since parsing failed
        assert!(matches!(
            events[0].data[0],
            AttributeValue::String(ref s) if s == "not a date"
        ));
    }

    #[test]
    fn test_max_input_size_limit() {
        let mut mapper = JsonSourceMapper::new();
        mapper.set_max_input_size(100); // Set small limit

        // Create JSON larger than limit
        let large_json = format!(r#"{{"data": "{}"}}"#, "x".repeat(200));
        let result = mapper.map(large_json.as_bytes());

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("exceeds maximum allowed"));
    }

    #[test]
    fn test_max_nesting_depth_limit() {
        let mut mapper = JsonSourceMapper::new();
        mapper.set_max_nesting_depth(3); // Set small limit

        let mut mappings = HashMap::new();
        // This path has 5 levels: a.b.c.d.e (exceeds limit of 3)
        mappings.insert("field".to_string(), "$.a.b.c.d.e".to_string());
        mapper = JsonSourceMapper::with_mappings(mappings);
        mapper.set_max_nesting_depth(3);

        let json_str = r#"{"a": {"b": {"c": {"d": {"e": "value"}}}}}"#;
        let result = mapper.map(json_str.as_bytes());

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("nesting depth"));
    }

    #[test]
    fn test_large_input_within_limit() {
        let mut mapper = JsonSourceMapper::new();
        mapper.set_max_input_size(1024 * 1024); // 1 MB limit

        // Create JSON within limit
        let json = format!(r#"{{"data": "{}"}}"#, "x".repeat(1000));
        let result = mapper.map(json.as_bytes());

        // Should succeed since it's within limit
        assert!(result.is_ok());
    }
}
