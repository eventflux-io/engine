// siddhi_rust/src/core/query/mod.rs

pub mod processor;
pub mod input; // For join stream runtimes and other input handling
// Other query submodules will be added here: input, output, selector (core internal versions)
pub mod output;   // For core query output components (callbacks, rate limiters)
pub mod selector; // For core query selector components (QuerySelector/SelectProcessor)
// pub mod stream; // This was for query_api::execution::query::input::stream, not core stream processors

// pub mod processor; // THIS IS THE DUPLICATE - REMOVING
// The first `pub mod processor;` at the top of the file is correct.

// For top-level query runtime classes like QueryRuntime, OnDemandQueryRuntime
// pub mod query_runtime;
// pub mod on_demand_query_runtime;
pub mod query_runtime; // Added
// etc.

// Re-export items from the processor and selector modules
pub use self::processor::{Processor, ProcessingMode, CommonProcessorMeta, FilterProcessor};
pub use self::selector::{SelectProcessor, OutputAttributeProcessor}; // Kept one
pub use self::query_runtime::QueryRuntime; // Added
pub use self::input::stream::join::{JoinProcessor, JoinStreamRuntime, JoinSide, JoinProcessorSide};
pub use self::input::stream::state::{SequenceProcessor, SequenceProcessorSide, SequenceType};
