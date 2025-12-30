// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Reference Compatibility Tests
// These tests are ported from the reference CEP implementation to ensure
// feature parity and behavioral compatibility.
//
// Test naming convention: {category}_{feature}_{test_number}
// Each test documents its reference source in comments.

#[path = "../common/mod.rs"]
pub mod common;

pub mod aggregations;
pub mod filters;
pub mod functions;
pub mod joins;
pub mod partitions;
pub mod patterns;
pub mod tables;
pub mod triggers;
pub mod windows;
