// SPDX-License-Identifier: MIT OR Apache-2.0

// eventflux_rust/src/core/mod.rs

// Top-level files from Java io.eventflux.core
pub mod eventflux_app_runtime; // For EventFluxAppRuntime.java (and Impl)
pub mod eventflux_app_runtime_builder;
pub mod eventflux_manager; // For EventFluxManager.java // Declare the module

// Sub-packages, corresponding to Java packages
pub mod aggregation;
pub mod config;
pub mod data_source;
pub mod debugger;
pub mod distributed; // Added for distributed processing
pub mod error; // Core error types for factory system and runtime
pub mod event;
pub mod exception; // For custom core-specific error types
pub mod executor;
pub mod extension;
pub mod function; // For UDFs like Script.java
pub mod partition;
pub mod persistence; // Added
pub mod query;
pub mod store;
pub mod stream;
pub mod table;
pub mod trigger;
pub mod util;
pub mod window;

// Re-export key public-facing structs from core
pub use self::data_source::{DataSource, DataSourceConfig, SqliteDataSource};
pub use self::error::{EventFluxError, EventFluxResult};
pub use self::eventflux_app_runtime::EventFluxAppRuntime;
pub use self::eventflux_app_runtime_builder::EventFluxAppRuntimeBuilder;
pub use self::eventflux_manager::EventFluxManager; // Added
                                                   // Other important re-exports will be added as these modules are built out.
