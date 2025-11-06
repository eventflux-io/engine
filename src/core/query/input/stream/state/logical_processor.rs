// SPDX-License-Identifier: MIT OR Apache-2.0

//! # DEPRECATED: Streaming Join Architecture (Not Pattern Processing)
//!
//! **WARNING**: This module implements streaming joins, NOT pattern processing.
//! It uses StreamEvent instead of StateEvent and lacks processor chaining.
//!
//! **Use Instead**:
//! - `logical_pre_state_processor.rs` - Correct pattern processing architecture
//! - `logical_post_state_processor.rs` - Correct pattern processing architecture
//!
//! **Will Be Removed**: Phase 2 (after migration complete)
//!
//! This file is kept temporarily for reference but should NOT be used in new code.

use log::error;
use std::sync::{Arc, Mutex};

use super::sequence_processor::SequenceSide;
use crate::core::config::eventflux_app_context::EventFluxAppContext;
use crate::core::config::eventflux_query_context::EventFluxQueryContext;
use crate::core::event::complex_event::ComplexEvent;
use crate::core::event::stream::{
    stream_event::StreamEvent, stream_event_cloner::StreamEventCloner,
    stream_event_factory::StreamEventFactory,
};
use crate::core::event::value::AttributeValue;
use crate::core::query::processor::{CommonProcessorMeta, ProcessingMode, Processor};

/// **DEPRECATED**: Use LogicalPreStateProcessor instead.
/// This enum is part of the old streaming join architecture.
#[deprecated(
    since = "0.1.0",
    note = "Use logical_pre_state_processor::LogicalType instead"
)]
#[derive(Debug, Clone, Copy)]
pub enum LogicalType {
    And,
    Or,
}

/// **DEPRECATED**: Use LogicalPreStateProcessor and LogicalPostStateProcessor instead.
/// This struct implements streaming joins, not pattern processing.
#[deprecated(
    since = "0.1.0",
    note = "Use LogicalPreStateProcessor for pattern processing"
)]
#[derive(Debug)]
pub struct LogicalProcessor {
    meta: CommonProcessorMeta,
    pub logical_type: LogicalType,
    pub first_buffer: Vec<StreamEvent>,
    pub second_buffer: Vec<StreamEvent>,
    pub first_attr_count: usize,
    pub second_attr_count: usize,
    pub next_processor: Option<Arc<Mutex<dyn Processor>>>,
    first_cloner: Option<StreamEventCloner>,
    second_cloner: Option<StreamEventCloner>,
    event_factory: StreamEventFactory,
}

impl LogicalProcessor {
    pub fn new(
        logical_type: LogicalType,
        first_attr_count: usize,
        second_attr_count: usize,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
    ) -> Self {
        Self {
            meta: CommonProcessorMeta::new(app_ctx, query_ctx),
            logical_type,
            first_buffer: Vec::new(),
            second_buffer: Vec::new(),
            first_attr_count,
            second_attr_count,
            next_processor: None,
            first_cloner: None,
            second_cloner: None,
            event_factory: StreamEventFactory::new(first_attr_count + second_attr_count, 0, 0),
        }
    }

    fn build_joined_event(
        &self,
        first: Option<&StreamEvent>,
        second: Option<&StreamEvent>,
    ) -> StreamEvent {
        let mut event = self.event_factory.new_instance();
        event.timestamp = second
            .map(|s| s.timestamp)
            .or_else(|| first.map(|f| f.timestamp))
            .unwrap_or(0); // Default to 0 if both are None (should not happen in normal operation)
        for i in 0..self.first_attr_count {
            let val = first
                .and_then(|f| f.before_window_data.get(i).cloned())
                .unwrap_or(AttributeValue::Null);
            event.before_window_data[i] = val;
        }
        for j in 0..self.second_attr_count {
            let val = second
                .and_then(|s| s.before_window_data.get(j).cloned())
                .unwrap_or(AttributeValue::Null);
            event.before_window_data[self.first_attr_count + j] = val;
        }
        event
    }

    fn forward(&self, se: StreamEvent) {
        if let Some(ref next) = self.next_processor {
            match next.lock() {
                Ok(mut processor) => processor.process(Some(Box::new(se))),
                Err(e) => {
                    error!("Next processor mutex poisoned during forward: {}", e);
                    // Skip forwarding - event lost
                }
            }
        }
    }

    fn try_produce(&mut self) {
        match self.logical_type {
            LogicalType::And => {
                while !self.first_buffer.is_empty() && !self.second_buffer.is_empty() {
                    let first = self.first_buffer.remove(0);
                    let second = self.second_buffer.remove(0);
                    let joined = self.build_joined_event(Some(&first), Some(&second));
                    self.forward(joined);
                }
            }
            LogicalType::Or => {
                while !self.first_buffer.is_empty() {
                    let first = self.first_buffer.remove(0);
                    let joined = self.build_joined_event(Some(&first), None);
                    self.forward(joined);
                }
                while !self.second_buffer.is_empty() {
                    let second = self.second_buffer.remove(0);
                    let joined = self.build_joined_event(None, Some(&second));
                    self.forward(joined);
                }
            }
        }
    }

    fn process_event(&mut self, side: SequenceSide, mut chunk: Option<Box<dyn ComplexEvent>>) {
        while let Some(mut ce) = chunk {
            chunk = ce.set_next(None);
            if let Some(se) = ce.as_any().downcast_ref::<StreamEvent>() {
                // Initialize cloner if needed
                match side {
                    SequenceSide::First => {
                        if self.first_cloner.is_none() {
                            self.first_cloner = Some(StreamEventCloner::from_event(se));
                        }
                        // Clone event using the initialized cloner
                        if let Some(ref cloner) = self.first_cloner {
                            let se_clone = cloner.copy_stream_event(se);
                            self.first_buffer.push(se_clone);
                        }
                    }
                    SequenceSide::Second => {
                        if self.second_cloner.is_none() {
                            self.second_cloner = Some(StreamEventCloner::from_event(se));
                        }
                        // Clone event using the initialized cloner
                        if let Some(ref cloner) = self.second_cloner {
                            let se_clone = cloner.copy_stream_event(se);
                            self.second_buffer.push(se_clone);
                        }
                    }
                }
                self.try_produce();
            }
        }
    }

    pub fn create_side_processor(
        self_arc: &Arc<Mutex<Self>>,
        side: SequenceSide,
    ) -> Arc<Mutex<LogicalProcessorSide>> {
        Arc::new(Mutex::new(LogicalProcessorSide {
            parent: Arc::clone(self_arc),
            side,
        }))
    }
}

#[derive(Debug)]
pub struct LogicalProcessorSide {
    parent: Arc<Mutex<LogicalProcessor>>,
    side: SequenceSide,
}

impl Processor for LogicalProcessorSide {
    fn process(&self, chunk: Option<Box<dyn ComplexEvent>>) {
        match self.parent.lock() {
            Ok(mut parent) => parent.process_event(self.side, chunk),
            Err(e) => {
                error!(
                    "LogicalProcessor parent mutex poisoned during process: {}",
                    e
                );
                // Cannot process - event lost
            }
        }
    }

    fn next_processor(&self) -> Option<Arc<Mutex<dyn Processor>>> {
        match self.parent.lock() {
            Ok(parent) => parent.next_processor.clone(),
            Err(e) => {
                error!(
                    "LogicalProcessor parent mutex poisoned during next_processor: {}",
                    e
                );
                None
            }
        }
    }

    fn set_next_processor(&mut self, next: Option<Arc<Mutex<dyn Processor>>>) {
        match self.parent.lock() {
            Ok(mut parent) => parent.next_processor = next,
            Err(e) => {
                error!(
                    "LogicalProcessor parent mutex poisoned during set_next_processor: {}",
                    e
                );
                // Cannot set - skip operation
            }
        }
    }

    fn clone_processor(&self, ctx: &Arc<EventFluxQueryContext>) -> Box<dyn Processor> {
        match self.parent.lock() {
            Ok(parent) => {
                let cloned = LogicalProcessor::new(
                    parent.logical_type,
                    parent.first_attr_count,
                    parent.second_attr_count,
                    Arc::clone(&parent.meta.eventflux_app_context),
                    Arc::clone(ctx),
                );
                let arc = Arc::new(Mutex::new(cloned));
                Box::new(LogicalProcessorSide {
                    parent: arc,
                    side: self.side,
                })
            }
            Err(e) => {
                error!(
                    "LogicalProcessor parent mutex poisoned during clone_processor: {}",
                    e
                );
                // Return a minimal clone with default values
                let app_ctx = Arc::new(EventFluxAppContext::new(
                    Arc::new(crate::core::config::eventflux_context::EventFluxContext::new()),
                    "default".to_string(),
                    Arc::new(crate::query_api::eventflux_app::EventFluxApp::new(
                        "default".to_string(),
                    )),
                    String::new(),
                ));
                let cloned =
                    LogicalProcessor::new(LogicalType::And, 0, 0, app_ctx, Arc::clone(ctx));
                let arc = Arc::new(Mutex::new(cloned));
                Box::new(LogicalProcessorSide {
                    parent: arc,
                    side: self.side,
                })
            }
        }
    }

    fn get_eventflux_app_context(&self) -> Arc<EventFluxAppContext> {
        match self.parent.lock() {
            Ok(parent) => parent.meta.eventflux_app_context.clone(),
            Err(e) => {
                error!(
                    "LogicalProcessor parent mutex poisoned during get_eventflux_app_context: {}",
                    e
                );
                // Return a default context
                Arc::new(EventFluxAppContext::new(
                    Arc::new(crate::core::config::eventflux_context::EventFluxContext::new()),
                    "default".to_string(),
                    Arc::new(crate::query_api::eventflux_app::EventFluxApp::new(
                        "default".to_string(),
                    )),
                    String::new(),
                ))
            }
        }
    }

    fn get_eventflux_query_context(&self) -> Arc<EventFluxQueryContext> {
        match self.parent.lock() {
            Ok(parent) => parent.meta.get_eventflux_query_context(),
            Err(e) => {
                error!(
                    "LogicalProcessor parent mutex poisoned during get_eventflux_query_context: {}",
                    e
                );
                // Return a minimal context
                let app_ctx = Arc::new(EventFluxAppContext::new(
                    Arc::new(crate::core::config::eventflux_context::EventFluxContext::new()),
                    "default".to_string(),
                    Arc::new(crate::query_api::eventflux_app::EventFluxApp::new(
                        "default".to_string(),
                    )),
                    String::new(),
                ));
                Arc::new(EventFluxQueryContext::new(
                    app_ctx,
                    "default".to_string(),
                    None,
                ))
            }
        }
    }

    fn get_processing_mode(&self) -> ProcessingMode {
        ProcessingMode::DEFAULT
    }

    fn is_stateful(&self) -> bool {
        true
    }
}
