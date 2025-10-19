// SPDX-License-Identifier: MIT OR Apache-2.0

//! M4: 3-Phase Validation System
//!
//! Provides comprehensive validation for EventFlux applications across three phases:
//! - Phase 1: Parse-Time (Syntax Only) - During SQL parsing and TOML loading
//! - Phase 2: Application Initialization (Fail-Fast) - Before processing events
//! - Phase 3: Runtime (Resilient Retry) - During event processing
//!
//! This module implements validation logic for:
//! - Circular dependency detection in stream queries
//! - DLQ (Dead Letter Queue) schema validation
//! - DLQ stream name and recursive restriction validation

pub mod circular_dependency;
pub mod dlq_validation;
pub mod query_helpers;

pub use circular_dependency::detect_circular_dependencies;
pub use dlq_validation::{
    validate_dlq_schema, validate_dlq_stream_name, validate_no_recursive_dlq,
};
pub use query_helpers::QuerySourceExtractor;
