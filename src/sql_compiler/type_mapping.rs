// SPDX-License-Identifier: MIT OR Apache-2.0

//! Type Mapping - SQL Types to AttributeType Conversion
//!
//! Maps SQL data types to EventFlux's AttributeType system.

use crate::query_api::definition::attribute::Type as AttributeType;
use sqlparser::ast::DataType;

use super::error::TypeError;

/// Convert SQL DataType to AttributeType
pub fn sql_type_to_attribute_type(sql_type: &DataType) -> Result<AttributeType, TypeError> {
    match sql_type {
        // String types
        DataType::Varchar(_) | DataType::Text | DataType::String(_) => Ok(AttributeType::STRING),
        DataType::Char(_) | DataType::CharacterVarying(_) => Ok(AttributeType::STRING),

        // Integer types
        DataType::Int(_) | DataType::Integer(_) | DataType::Int2(_) | DataType::Int4(_) => {
            Ok(AttributeType::INT)
        }
        DataType::SmallInt(_) | DataType::TinyInt(_) => Ok(AttributeType::INT),

        // Long types
        DataType::BigInt(_) | DataType::Int8(_) => Ok(AttributeType::LONG),

        // Float types
        DataType::Float(_) | DataType::Real => Ok(AttributeType::FLOAT),

        // Double types
        DataType::Double | DataType::DoublePrecision => Ok(AttributeType::DOUBLE),

        // Boolean types
        DataType::Boolean => Ok(AttributeType::BOOL),

        // Decimal types (precision loss warning)
        DataType::Decimal(_) | DataType::Numeric(_) => {
            // TODO: Add proper logging when log crate is configured
            // For M1, just map to DOUBLE silently
            Ok(AttributeType::DOUBLE)
        }

        // Timestamp types (map to LONG as Unix millis)
        DataType::Timestamp(_, _) | DataType::Datetime(_) => Ok(AttributeType::LONG),
        DataType::Date => Ok(AttributeType::LONG),
        DataType::Time(_, _) => Ok(AttributeType::LONG),

        // Unsupported types
        DataType::Array(_) => Err(TypeError::UnsupportedType(
            "ARRAY types not supported in M1".to_string(),
        )),
        DataType::Struct(_) => Err(TypeError::UnsupportedType(
            "STRUCT types not supported in M1".to_string(),
        )),
        DataType::JSON => Err(TypeError::UnsupportedType(
            "JSON type not supported in M1".to_string(),
        )),
        DataType::Binary(_) | DataType::Varbinary(_) | DataType::Blob(_) => Err(
            TypeError::UnsupportedType("Binary types not supported in M1".to_string()),
        ),

        // Catch-all for other types
        other => Err(TypeError::UnsupportedType(format!("{:?}", other))),
    }
}

/// Convert AttributeType back to SQL DataType (for reverse mapping if needed)
pub fn attribute_type_to_sql_type(attr_type: &AttributeType) -> DataType {
    match attr_type {
        AttributeType::STRING => DataType::Varchar(None),
        AttributeType::INT => DataType::Int(None),
        AttributeType::LONG => DataType::BigInt(None),
        AttributeType::FLOAT => DataType::Float(None),
        AttributeType::DOUBLE => DataType::Double,
        AttributeType::BOOL => DataType::Boolean,
        AttributeType::OBJECT => DataType::JSON, // Best approximation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_types() {
        assert_eq!(
            sql_type_to_attribute_type(&DataType::Text).unwrap(),
            AttributeType::STRING
        );
    }

    #[test]
    fn test_integer_types() {
        assert_eq!(
            sql_type_to_attribute_type(&DataType::Int(None)).unwrap(),
            AttributeType::INT
        );
        assert_eq!(
            sql_type_to_attribute_type(&DataType::SmallInt(None)).unwrap(),
            AttributeType::INT
        );
    }

    #[test]
    fn test_long_types() {
        assert_eq!(
            sql_type_to_attribute_type(&DataType::BigInt(None)).unwrap(),
            AttributeType::LONG
        );
    }

    #[test]
    fn test_float_types() {
        assert_eq!(
            sql_type_to_attribute_type(&DataType::Float(None)).unwrap(),
            AttributeType::FLOAT
        );
        assert_eq!(
            sql_type_to_attribute_type(&DataType::Real).unwrap(),
            AttributeType::FLOAT
        );
    }

    #[test]
    fn test_double_types() {
        assert_eq!(
            sql_type_to_attribute_type(&DataType::Double).unwrap(),
            AttributeType::DOUBLE
        );
        assert_eq!(
            sql_type_to_attribute_type(&DataType::DoublePrecision).unwrap(),
            AttributeType::DOUBLE
        );
    }

    #[test]
    fn test_boolean_type() {
        assert_eq!(
            sql_type_to_attribute_type(&DataType::Boolean).unwrap(),
            AttributeType::BOOL
        );
    }

    #[test]
    fn test_reverse_mapping() {
        assert_eq!(
            attribute_type_to_sql_type(&AttributeType::STRING),
            DataType::Varchar(None)
        );
        assert_eq!(
            attribute_type_to_sql_type(&AttributeType::INT),
            DataType::Int(None)
        );
        assert_eq!(
            attribute_type_to_sql_type(&AttributeType::DOUBLE),
            DataType::Double
        );
    }
}
