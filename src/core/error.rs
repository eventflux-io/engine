// SPDX-License-Identifier: MIT OR Apache-2.0

//! EventFlux Core Error Types
//!
//! Comprehensive error handling for EventFlux runtime operations.

use thiserror::Error;

/// Result type for EventFlux operations
pub type EventFluxResult<T> = Result<T, EventFluxError>;

/// Comprehensive EventFlux error types
#[derive(Error, Debug)]
pub enum EventFluxError {
    #[error("Configuration error: {message}")]
    Configuration {
        message: String,
        config_key: Option<String>,
    },

    #[error("Invalid parameter '{parameter:?}': {message}")]
    InvalidParameter {
        message: String,
        parameter: Option<String>,
        expected: Option<String>,
    },

    #[error("Extension '{extension_type}:{name}' not found")]
    ExtensionNotFound {
        extension_type: String,
        name: String,
    },

    #[error("Connection unavailable: {message}")]
    ConnectionUnavailable {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Initialization failed: {message}")]
    InitializationFailed {
        message: String,
        component: Option<String>,
    },

    #[error("Validation failed: {message}")]
    ValidationFailed {
        message: String,
        field: Option<String>,
    },

    #[error("Format '{format}' not supported by extension '{extension}'")]
    UnsupportedFormat { format: String, extension: String },

    #[error("Missing required parameter: {parameter}")]
    MissingParameter { parameter: String },

    #[error("Runtime error: {message}")]
    Runtime {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

// Custom error creation helpers
impl EventFluxError {
    /// Create a configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
            config_key: None,
        }
    }

    /// Create a configuration error with a specific key
    pub fn configuration_with_key(message: impl Into<String>, config_key: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
            config_key: Some(config_key.into()),
        }
    }

    /// Create an invalid parameter error
    pub fn invalid_parameter(message: impl Into<String>) -> Self {
        Self::InvalidParameter {
            message: message.into(),
            parameter: None,
            expected: None,
        }
    }

    /// Create an invalid parameter error with details
    pub fn invalid_parameter_with_details(
        message: impl Into<String>,
        parameter: impl Into<String>,
        expected: impl Into<String>,
    ) -> Self {
        Self::InvalidParameter {
            message: message.into(),
            parameter: Some(parameter.into()),
            expected: Some(expected.into()),
        }
    }

    /// Create an extension not found error
    pub fn extension_not_found(extension_type: impl Into<String>, name: impl Into<String>) -> Self {
        Self::ExtensionNotFound {
            extension_type: extension_type.into(),
            name: name.into(),
        }
    }

    /// Create a connection unavailable error
    pub fn connection_unavailable(message: impl Into<String>) -> Self {
        Self::ConnectionUnavailable {
            message: message.into(),
            source: None,
        }
    }

    /// Create a connection unavailable error with source
    pub fn connection_unavailable_with_source(
        message: impl Into<String>,
        source: Box<dyn std::error::Error + Send + Sync>,
    ) -> Self {
        Self::ConnectionUnavailable {
            message: message.into(),
            source: Some(source),
        }
    }

    /// Create an initialization failed error
    pub fn initialization_failed(message: impl Into<String>) -> Self {
        Self::InitializationFailed {
            message: message.into(),
            component: None,
        }
    }

    /// Create an initialization failed error with component
    pub fn initialization_failed_with_component(
        message: impl Into<String>,
        component: impl Into<String>,
    ) -> Self {
        Self::InitializationFailed {
            message: message.into(),
            component: Some(component.into()),
        }
    }

    /// Create a validation failed error
    pub fn validation_failed(message: impl Into<String>) -> Self {
        Self::ValidationFailed {
            message: message.into(),
            field: None,
        }
    }

    /// Create an unsupported format error
    pub fn unsupported_format(format: impl Into<String>, extension: impl Into<String>) -> Self {
        Self::UnsupportedFormat {
            format: format.into(),
            extension: extension.into(),
        }
    }

    /// Create a missing parameter error
    pub fn missing_parameter(parameter: impl Into<String>) -> Self {
        Self::MissingParameter {
            parameter: parameter.into(),
        }
    }

    /// Create a runtime error
    pub fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime {
            message: message.into(),
            source: None,
        }
    }

    /// Create a generic error from a string
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configuration_error() {
        let error = EventFluxError::configuration("test error");
        assert!(matches!(error, EventFluxError::Configuration { .. }));
    }

    #[test]
    fn test_extension_not_found_error() {
        let error = EventFluxError::extension_not_found("source", "kafka");
        assert!(matches!(error, EventFluxError::ExtensionNotFound { .. }));
    }

    #[test]
    fn test_missing_parameter_error() {
        let error = EventFluxError::missing_parameter("kafka.brokers");
        assert!(matches!(error, EventFluxError::MissingParameter { .. }));
    }

    #[test]
    fn test_unsupported_format_error() {
        let error = EventFluxError::unsupported_format("xml", "kafka");
        assert!(matches!(error, EventFluxError::UnsupportedFormat { .. }));
    }
}
