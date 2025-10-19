// SPDX-License-Identifier: MIT OR Apache-2.0

pub mod error;
pub use error::{ErrorCategory, EventFluxError, EventFluxResult, IntoEventFluxResult};
// ErrorStore types now live under the stream::output module
pub use crate::core::stream::output::error_store::{ErrorStore, InMemoryErrorStore};
