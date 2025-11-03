// SPDX-License-Identifier: MIT OR Apache-2.0

//! # DEPRECATED: Temporal Join Architecture (Not Pattern Processing)
//!
//! **WARNING**: This module implements temporal joins, NOT pattern processing.
//! It uses StreamEvent instead of StateEvent and lacks processor chaining.
//!
//! **Use Instead**:
//! - `stream_pre_state_processor.rs` - Correct pattern processing with StateEvent
//! - `stream_post_state_processor.rs` - Correct pattern processing architecture
//!
//! **Will Be Removed**: Phase 2 (after migration complete)
//!
//! This file is kept temporarily for reference but should NOT be used in new code.

use log::error;
use std::sync::{Arc, Mutex};

use crate::core::config::eventflux_app_context::EventFluxAppContext;
use crate::core::config::eventflux_query_context::EventFluxQueryContext;
use crate::core::event::complex_event::ComplexEvent;
use crate::core::event::stream::{
    stream_event::StreamEvent, stream_event_cloner::StreamEventCloner,
    stream_event_factory::StreamEventFactory,
};
use crate::core::event::value::AttributeValue;
use crate::core::query::processor::{CommonProcessorMeta, ProcessingMode, Processor};

/// **DEPRECATED**: Use StreamPreStateProcessor::StateType instead.
/// This enum is part of the old temporal join architecture.
#[deprecated(
    since = "0.1.0",
    note = "Use stream_pre_state_processor::StateType instead"
)]
#[derive(Debug, Clone, Copy)]
pub enum SequenceType {
    Pattern,
    Sequence,
}

/// **DEPRECATED**: Use StreamPreStateProcessor and StreamPostStateProcessor instead.
/// This struct implements temporal joins, not pattern processing.
#[deprecated(
    since = "0.1.0",
    note = "Use StreamPreStateProcessor for pattern processing"
)]
#[derive(Debug)]
pub struct SequenceProcessor {
    meta: CommonProcessorMeta,
    pub sequence_type: SequenceType,
    pub first_buffer: Vec<StreamEvent>,
    pub second_buffer: Vec<StreamEvent>,
    pub first_attr_count: usize,
    pub second_attr_count: usize,
    pub next_processor: Option<Arc<Mutex<dyn Processor>>>,
    first_cloner: Option<StreamEventCloner>,
    second_cloner: Option<StreamEventCloner>,
    event_factory: StreamEventFactory,
    pub first_min: i32,
    pub first_max: i32,
    pub second_min: i32,
    pub second_max: i32,
    pub within_time: Option<i64>,
}

impl SequenceProcessor {
    pub fn new(
        sequence_type: SequenceType,
        first_attr_count: usize,
        second_attr_count: usize,
        first_min: i32,
        first_max: i32,
        second_min: i32,
        second_max: i32,
        within_time: Option<i64>,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
    ) -> Self {
        Self {
            meta: CommonProcessorMeta::new(app_ctx, query_ctx),
            sequence_type,
            first_buffer: Vec::new(),
            second_buffer: Vec::new(),
            first_attr_count,
            second_attr_count,
            next_processor: None,
            first_cloner: None,
            second_cloner: None,
            event_factory: StreamEventFactory::new(first_attr_count + second_attr_count, 0, 0),
            first_min,
            first_max,
            second_min,
            second_max,
            within_time,
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
            .unwrap_or_else(|| first.map(|f| f.timestamp).unwrap_or(0));
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

    fn check_and_produce(&mut self) {
        while !self.second_buffer.is_empty() {
            if (self.first_buffer.len() as i32) < self.first_min {
                break;
            }
            let second = self.second_buffer.remove(0);

            if self.first_min == 0 && (self.within_time.is_none() || self.first_buffer.is_empty()) {
                let joined = self.build_joined_event(None, Some(&second));
                self.forward(joined);
            }

            let max = if self.first_max < 0 {
                self.first_buffer.len()
            } else {
                usize::min(self.first_max as usize, self.first_buffer.len())
            };
            let start_idx = self.first_buffer.len() - max;
            for i in start_idx..self.first_buffer.len() {
                let first = &self.first_buffer[i];
                if let Some(wt) = self.within_time {
                    if second.timestamp - first.timestamp > wt {
                        continue;
                    }
                }
                let joined = self.build_joined_event(Some(first), Some(&second));
                self.forward(joined);
            }

            if matches!(self.sequence_type, SequenceType::Sequence) {
                self.first_buffer.clear();
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
                            if let Some(wt) = self.within_time {
                                self.first_buffer
                                    .retain(|e| se.timestamp - e.timestamp <= wt);
                            }
                            if matches!(self.sequence_type, SequenceType::Pattern) {
                                self.check_and_produce();
                            }
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
                            if let Some(wt) = self.within_time {
                                self.second_buffer
                                    .retain(|e| se.timestamp - e.timestamp <= wt);
                            }
                            self.check_and_produce();
                        }
                    }
                }
            }
        }
    }

    pub fn create_side_processor(
        self_arc: &Arc<Mutex<Self>>,
        side: SequenceSide,
    ) -> Arc<Mutex<SequenceProcessorSide>> {
        Arc::new(Mutex::new(SequenceProcessorSide {
            parent: Arc::clone(self_arc),
            side,
        }))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SequenceSide {
    First,
    Second,
}

#[derive(Debug)]
pub struct SequenceProcessorSide {
    parent: Arc<Mutex<SequenceProcessor>>,
    side: SequenceSide,
}

impl Processor for SequenceProcessorSide {
    fn process(&self, chunk: Option<Box<dyn ComplexEvent>>) {
        match self.parent.lock() {
            Ok(mut parent) => parent.process_event(self.side, chunk),
            Err(e) => {
                error!(
                    "SequenceProcessor parent mutex poisoned during process: {}",
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
                    "SequenceProcessor parent mutex poisoned during next_processor: {}",
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
                    "SequenceProcessor parent mutex poisoned during set_next_processor: {}",
                    e
                );
                // Cannot set - skip operation
            }
        }
    }

    fn clone_processor(&self, ctx: &Arc<EventFluxQueryContext>) -> Box<dyn Processor> {
        match self.parent.lock() {
            Ok(parent) => {
                let cloned = SequenceProcessor::new(
                    parent.sequence_type,
                    parent.first_attr_count,
                    parent.second_attr_count,
                    parent.first_min,
                    parent.first_max,
                    parent.second_min,
                    parent.second_max,
                    parent.within_time,
                    Arc::clone(&parent.meta.eventflux_app_context),
                    Arc::clone(ctx),
                );
                let arc = Arc::new(Mutex::new(cloned));
                let side = SequenceProcessorSide {
                    parent: arc,
                    side: self.side,
                };
                Box::new(side)
            }
            Err(e) => {
                error!(
                    "SequenceProcessor parent mutex poisoned during clone_processor: {}",
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
                let cloned = SequenceProcessor::new(
                    SequenceType::Sequence,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    None,
                    app_ctx,
                    Arc::clone(ctx),
                );
                let arc = Arc::new(Mutex::new(cloned));
                Box::new(SequenceProcessorSide {
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
                    "SequenceProcessor parent mutex poisoned during get_eventflux_app_context: {}",
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
                error!("SequenceProcessor parent mutex poisoned during get_eventflux_query_context: {}", e);
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
