// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::core::config::{
    eventflux_app_context::EventFluxAppContext, eventflux_query_context::EventFluxQueryContext,
};
use crate::core::event::complex_event::ComplexEvent;
use crate::core::event::complex_event::ComplexEventType;
use crate::core::event::stream::stream_event::StreamEvent;
use crate::core::extension::WindowProcessorFactory;
use crate::core::query::processor::{CommonProcessorMeta, ProcessingMode, Processor};
use crate::core::util::scheduler::{Schedulable, Scheduler};
use crate::query_api::execution::query::input::handler::WindowHandler;
use crate::query_api::expression::{constant::ConstantValueWithFloat, Expression};
use std::collections::VecDeque;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

// Import StateHolder trait and related types
use crate::core::persistence::state_holder::{
    AccessPattern, ChangeLog, CheckpointId, SchemaVersion, SerializationHints, StateError,
    StateHolder as NewStateHolder, StateMetadata, StateSize, StateSnapshot,
};

// Window type constants
pub mod types;

// Import changelog helpers
pub(crate) mod changelog_helpers;

// Import session window processor
mod session_window_processor;
use session_window_processor::SessionWindowProcessor;

// Import session window state holder
mod session_window_state_holder;

// Import sort window processor
mod sort_window_processor;
use sort_window_processor::SortWindowProcessor;

// Import enhanced length window state holder
mod length_window_state_holder;
use length_window_state_holder::LengthWindowStateHolder;

// Import enhanced time window state holder
mod time_window_state_holder;
use time_window_state_holder::TimeWindowStateHolder;

// Import enhanced length batch window state holder
mod length_batch_window_state_holder;
use length_batch_window_state_holder::LengthBatchWindowStateHolder;

// Import enhanced time batch window state holder
mod time_batch_window_state_holder;
use time_batch_window_state_holder::TimeBatchWindowStateHolder;

// Import enhanced external time window state holder
mod external_time_window_state_holder;

pub trait WindowProcessor: Processor {}

#[derive(Debug)]
pub struct LengthWindowProcessor {
    meta: CommonProcessorMeta,
    pub length: usize,
    buffer: Arc<Mutex<VecDeque<Arc<StreamEvent>>>>,
}

impl LengthWindowProcessor {
    pub fn new(
        length: usize,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
    ) -> Self {
        let buffer = Arc::new(Mutex::new(VecDeque::new()));

        // Create enhanced StateHolder and register it for persistence
        let component_id = format!("length_window_{}_{}", query_ctx.get_name(), length);
        let state_holder = Arc::new(LengthWindowStateHolder::new(
            Arc::clone(&buffer),
            component_id.clone(),
            length,
        ));

        // Register state holder with SnapshotService for persistence
        let state_holder_clone = (*state_holder).clone();
        let state_holder_arc: Arc<Mutex<dyn crate::core::persistence::StateHolder>> =
            Arc::new(Mutex::new(state_holder_clone));
        if let Some(snapshot_service) = app_ctx.get_snapshot_service() {
            snapshot_service.register_state_holder(component_id, state_holder_arc);
        }

        Self {
            meta: CommonProcessorMeta::new(app_ctx, query_ctx),
            length,
            buffer,
        }
    }

    pub fn from_handler(
        handler: &WindowHandler,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
        _parse_ctx: &crate::core::util::parser::expression_parser::ExpressionParserContext,
    ) -> Result<Self, String> {
        let expr = handler
            .get_parameters()
            .first()
            .ok_or("Length window requires a parameter")?;
        if let Expression::Constant(c) = expr {
            let len = match &c.value {
                ConstantValueWithFloat::Int(i) => *i as usize,
                ConstantValueWithFloat::Long(l) => *l as usize,
                _ => return Err("Length window size must be int or long".to_string()),
            };
            Ok(Self::new(len, app_ctx, query_ctx))
        } else {
            Err("Length window size must be constant".to_string())
        }
    }
}

impl Processor for LengthWindowProcessor {
    fn process(&self, complex_event_chunk: Option<Box<dyn ComplexEvent>>) {
        if let Some(ref next) = self.meta.next_processor {
            if let Some(ref chunk) = complex_event_chunk {
                let mut current_opt = Some(chunk.as_ref() as &dyn ComplexEvent);
                while let Some(ev) = current_opt {
                    if let Some(se) = ev.as_any().downcast_ref::<StreamEvent>() {
                        let mut expired: Option<Box<dyn ComplexEvent>> = None;
                        {
                            let mut buf = self.buffer.lock().unwrap();
                            if buf.len() >= self.length {
                                if let Some(old) = buf.pop_front() {
                                    let mut ex = old.as_ref().clone_without_next();
                                    ex.set_event_type(ComplexEventType::Expired);
                                    ex.set_timestamp(se.timestamp);
                                    expired = Some(Box::new(ex));
                                }
                            }
                            buf.push_back(Arc::new(se.clone_without_next()));
                        }
                        if let Some(mut ex) = expired {
                            let tail = ex.mut_next_ref_option();
                            *tail = Some(Box::new(se.clone_without_next()));
                            next.lock().unwrap().process(Some(ex));
                        } else {
                            next.lock()
                                .unwrap()
                                .process(Some(Box::new(se.clone_without_next())));
                        }
                    }
                    current_opt = ev.get_next();
                }
            } else {
                next.lock().unwrap().process(None);
            }
        }
    }

    fn next_processor(&self) -> Option<Arc<Mutex<dyn Processor>>> {
        self.meta.next_processor.as_ref().map(Arc::clone)
    }

    fn set_next_processor(&mut self, next: Option<Arc<Mutex<dyn Processor>>>) {
        self.meta.next_processor = next;
    }

    fn clone_processor(&self, query_ctx: &Arc<EventFluxQueryContext>) -> Box<dyn Processor> {
        Box::new(Self::new(
            self.length,
            Arc::clone(&self.meta.eventflux_app_context),
            Arc::clone(query_ctx),
        ))
    }

    fn get_eventflux_app_context(&self) -> Arc<EventFluxAppContext> {
        Arc::clone(&self.meta.eventflux_app_context)
    }
    fn get_eventflux_query_context(&self) -> Arc<EventFluxQueryContext> {
        self.meta.get_eventflux_query_context()
    }

    fn get_processing_mode(&self) -> ProcessingMode {
        ProcessingMode::SLIDE
    }

    fn is_stateful(&self) -> bool {
        true
    }
}

impl WindowProcessor for LengthWindowProcessor {}

#[derive(Debug)]
pub struct TimeWindowProcessor {
    meta: CommonProcessorMeta,
    pub duration_ms: i64,
    scheduler: Option<Arc<Scheduler>>,
    buffer: Arc<Mutex<VecDeque<Arc<StreamEvent>>>>,
    state_holder: Option<TimeWindowStateHolder>,
}

impl TimeWindowProcessor {
    pub fn new(
        duration_ms: i64,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
    ) -> Self {
        let scheduler = app_ctx.get_scheduler();
        let buffer = Arc::new(Mutex::new(VecDeque::new()));
        let component_id = format!("time_window_{}_{}", query_ctx.get_name(), duration_ms);

        let state_holder = Some(TimeWindowStateHolder::new(
            Arc::clone(&buffer),
            component_id,
            duration_ms,
        ));

        Self {
            meta: CommonProcessorMeta::new(app_ctx, query_ctx),
            duration_ms,
            scheduler,
            buffer,
            state_holder,
        }
    }

    pub fn from_handler(
        handler: &WindowHandler,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
        _parse_ctx: &crate::core::util::parser::expression_parser::ExpressionParserContext,
    ) -> Result<Self, String> {
        let expr = handler
            .get_parameters()
            .first()
            .ok_or("Time window requires a parameter")?;
        if let Expression::Constant(c) = expr {
            let dur = match &c.value {
                ConstantValueWithFloat::Time(t) => *t,
                ConstantValueWithFloat::Long(l) => *l,
                _ => return Err("Time window duration must be time constant".to_string()),
            };
            Ok(Self::new(dur, app_ctx, query_ctx))
        } else {
            Err("Time window duration must be constant".to_string())
        }
    }
}

#[derive(Debug, Clone)]
struct ExpireTask {
    event: Arc<StreamEvent>,
    buffer: Arc<Mutex<VecDeque<Arc<StreamEvent>>>>,
    next: Option<Arc<Mutex<dyn Processor>>>,
    state_holder: Option<TimeWindowStateHolder>,
}

impl Schedulable for ExpireTask {
    fn on_time(&self, timestamp: i64) {
        let ev_arc = {
            let mut buf = self.buffer.lock().unwrap();
            if let Some(pos) = buf.iter().position(|e| Arc::ptr_eq(e, &self.event)) {
                buf.remove(pos)
            } else {
                None
            }
        };
        if let Some(ev_arc) = ev_arc {
            let mut ev = ev_arc.as_ref().clone_without_next();
            ev.set_event_type(ComplexEventType::Expired);
            ev.set_timestamp(timestamp);

            // Track state change
            if let Some(ref state_holder) = self.state_holder {
                state_holder.record_event_expired(&ev);
            }

            if let Some(ref next) = self.next {
                next.lock().unwrap().process(Some(Box::new(ev)));
            }
        }
    }
}

#[derive(Clone)]
struct BatchFlushTask {
    buffer: Arc<Mutex<Vec<StreamEvent>>>,
    expired: Arc<Mutex<Vec<StreamEvent>>>,
    next: Option<Arc<Mutex<dyn Processor>>>,
    duration_ms: i64,
    scheduler: Arc<Scheduler>,
    start_time: Arc<Mutex<Option<i64>>>,
}

impl Schedulable for BatchFlushTask {
    fn on_time(&self, timestamp: i64) {
        let expired_batch: Vec<StreamEvent> = {
            let mut ex = self.expired.lock().unwrap();
            std::mem::take(&mut *ex)
        };
        let current_batch: Vec<StreamEvent> = {
            let mut buf = self.buffer.lock().unwrap();
            std::mem::take(&mut *buf)
        };

        if expired_batch.is_empty() && current_batch.is_empty() {
            *self.start_time.lock().unwrap() = Some(timestamp);
            self.scheduler
                .notify_at(timestamp + self.duration_ms, Arc::new(self.clone()));
            return;
        }

        let mut head: Option<Box<dyn ComplexEvent>> = None;
        let mut tail = &mut head;

        for mut e in expired_batch {
            e.set_event_type(ComplexEventType::Expired);
            e.set_timestamp(timestamp);
            *tail = Some(Box::new(e.clone_without_next()));
            tail = tail.as_mut().unwrap().mut_next_ref_option();
        }

        for e in &current_batch {
            *tail = Some(Box::new(e.clone_without_next()));
            tail = tail.as_mut().unwrap().mut_next_ref_option();
        }

        {
            let mut ex = self.expired.lock().unwrap();
            ex.extend(current_batch);
        }

        if let Some(chain) = head {
            if let Some(ref next) = self.next {
                next.lock().unwrap().process(Some(chain));
            }
        }

        *self.start_time.lock().unwrap() = Some(timestamp);
        self.scheduler
            .notify_at(timestamp + self.duration_ms, Arc::new(self.clone()));
    }
}

impl Processor for TimeWindowProcessor {
    fn process(&self, complex_event_chunk: Option<Box<dyn ComplexEvent>>) {
        if let Some(ref next) = self.meta.next_processor {
            if let Some(ref scheduler) = self.scheduler {
                if let Some(ref chunk) = complex_event_chunk {
                    let mut current_opt = Some(chunk.as_ref() as &dyn ComplexEvent);
                    while let Some(ev) = current_opt {
                        if let Some(se) = ev.as_any().downcast_ref::<StreamEvent>() {
                            let arc = Arc::new(se.clone_without_next());
                            {
                                let mut buf = self.buffer.lock().unwrap();
                                buf.push_back(Arc::clone(&arc));
                            }

                            // Track state change
                            if let Some(ref state_holder) = self.state_holder {
                                state_holder.record_event_added(se);
                            }

                            let task = ExpireTask {
                                event: Arc::clone(&arc),
                                buffer: Arc::clone(&self.buffer),
                                next: Some(Arc::clone(next)),
                                state_holder: self.state_holder.as_ref().cloned(),
                            };
                            scheduler.notify_at(se.timestamp + self.duration_ms, Arc::new(task));
                        }
                        current_opt = ev.get_next();
                    }
                }
            }
            next.lock().unwrap().process(complex_event_chunk);
        }
    }

    fn next_processor(&self) -> Option<Arc<Mutex<dyn Processor>>> {
        self.meta.next_processor.as_ref().map(Arc::clone)
    }

    fn set_next_processor(&mut self, next: Option<Arc<Mutex<dyn Processor>>>) {
        self.meta.next_processor = next;
    }

    fn clone_processor(&self, query_ctx: &Arc<EventFluxQueryContext>) -> Box<dyn Processor> {
        Box::new(Self::new(
            self.duration_ms,
            Arc::clone(&self.meta.eventflux_app_context),
            Arc::clone(query_ctx),
        ))
    }

    fn get_eventflux_app_context(&self) -> Arc<EventFluxAppContext> {
        Arc::clone(&self.meta.eventflux_app_context)
    }
    fn get_eventflux_query_context(&self) -> Arc<EventFluxQueryContext> {
        self.meta.get_eventflux_query_context()
    }

    fn get_processing_mode(&self) -> ProcessingMode {
        ProcessingMode::SLIDE
    }

    fn is_stateful(&self) -> bool {
        true
    }
}

impl WindowProcessor for TimeWindowProcessor {}

impl NewStateHolder for TimeWindowProcessor {
    fn schema_version(&self) -> crate::core::persistence::state_holder::SchemaVersion {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.schema_version()
        } else {
            crate::core::persistence::state_holder::SchemaVersion::new(1, 0, 0)
        }
    }

    fn serialize_state(
        &self,
        hints: &crate::core::persistence::state_holder::SerializationHints,
    ) -> Result<
        crate::core::persistence::state_holder::StateSnapshot,
        crate::core::persistence::state_holder::StateError,
    > {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.serialize_state(hints)
        } else {
            Err(
                crate::core::persistence::state_holder::StateError::InvalidStateData {
                    message: "StateHolder not initialized".to_string(),
                },
            )
        }
    }

    fn deserialize_state(
        &self,
        snapshot: &crate::core::persistence::state_holder::StateSnapshot,
    ) -> Result<(), crate::core::persistence::state_holder::StateError> {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.deserialize_state(snapshot)
        } else {
            Err(
                crate::core::persistence::state_holder::StateError::InvalidStateData {
                    message: "StateHolder not initialized".to_string(),
                },
            )
        }
    }

    fn get_changelog(
        &self,
        since: crate::core::persistence::state_holder::CheckpointId,
    ) -> Result<
        crate::core::persistence::state_holder::ChangeLog,
        crate::core::persistence::state_holder::StateError,
    > {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.get_changelog(since)
        } else {
            Err(
                crate::core::persistence::state_holder::StateError::InvalidStateData {
                    message: "StateHolder not initialized".to_string(),
                },
            )
        }
    }

    fn apply_changelog(
        &self,
        changes: &crate::core::persistence::state_holder::ChangeLog,
    ) -> Result<(), crate::core::persistence::state_holder::StateError> {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.apply_changelog(changes)
        } else {
            Err(
                crate::core::persistence::state_holder::StateError::InvalidStateData {
                    message: "StateHolder not initialized".to_string(),
                },
            )
        }
    }

    fn estimate_size(&self) -> crate::core::persistence::state_holder::StateSize {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.estimate_size()
        } else {
            crate::core::persistence::state_holder::StateSize {
                bytes: 0,
                entries: 0,
                estimated_growth_rate: 0.0,
            }
        }
    }

    fn access_pattern(&self) -> crate::core::persistence::state_holder::AccessPattern {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.access_pattern()
        } else {
            crate::core::persistence::state_holder::AccessPattern::Sequential
        }
    }

    fn component_metadata(&self) -> crate::core::persistence::state_holder::StateMetadata {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.component_metadata()
        } else {
            crate::core::persistence::state_holder::StateMetadata::new(
                format!("time_window_{}", self.duration_ms),
                "TimeWindowProcessor".to_string(),
            )
        }
    }
}

pub fn create_window_processor(
    handler: &WindowHandler,
    app_ctx: Arc<EventFluxAppContext>,
    query_ctx: Arc<EventFluxQueryContext>,
    parse_ctx: &crate::core::util::parser::expression_parser::ExpressionParserContext,
) -> Result<Arc<Mutex<dyn Processor>>, String> {
    // Look up window factory from registry
    if let Some(factory) = app_ctx
        .get_eventflux_context()
        .get_window_factory(&handler.name)
    {
        factory.create(handler, app_ctx, query_ctx, parse_ctx)
    } else {
        // Provide helpful error message with available window types
        let available = app_ctx.get_eventflux_context().list_window_factory_names();
        Err(format!(
            "Unknown window type '{}'. Available: {:?}",
            handler.name, available
        ))
    }
}

#[derive(Debug, Clone)]
pub struct LengthWindowFactory;

impl WindowProcessorFactory for LengthWindowFactory {
    fn name(&self) -> &'static str {
        "length"
    }
    fn create(
        &self,
        handler: &WindowHandler,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
        parse_ctx: &crate::core::util::parser::expression_parser::ExpressionParserContext,
    ) -> Result<Arc<Mutex<dyn Processor>>, String> {
        Ok(Arc::new(Mutex::new(LengthWindowProcessor::from_handler(
            handler, app_ctx, query_ctx, parse_ctx,
        )?)))
    }

    fn clone_box(&self) -> Box<dyn WindowProcessorFactory> {
        Box::new(Self {})
    }
}

#[derive(Debug, Clone)]
pub struct TimeWindowFactory;

impl WindowProcessorFactory for TimeWindowFactory {
    fn name(&self) -> &'static str {
        "time"
    }
    fn create(
        &self,
        handler: &WindowHandler,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
        parse_ctx: &crate::core::util::parser::expression_parser::ExpressionParserContext,
    ) -> Result<Arc<Mutex<dyn Processor>>, String> {
        Ok(Arc::new(Mutex::new(TimeWindowProcessor::from_handler(
            handler, app_ctx, query_ctx, parse_ctx,
        )?)))
    }

    fn clone_box(&self) -> Box<dyn WindowProcessorFactory> {
        Box::new(Self {})
    }
}

// ---- LengthBatchWindowProcessor ----

#[derive(Debug)]
pub struct LengthBatchWindowProcessor {
    meta: CommonProcessorMeta,
    pub length: usize,
    buffer: Arc<Mutex<Vec<StreamEvent>>>,
    expired: Arc<Mutex<Vec<StreamEvent>>>,
    state_holder: Option<LengthBatchWindowStateHolder>,
}

impl LengthBatchWindowProcessor {
    pub fn new(
        length: usize,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
    ) -> Self {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let expired = Arc::new(Mutex::new(Vec::new()));

        // Create enhanced StateHolder and register it
        let component_id = format!("length_batch_window_{}_{}", query_ctx.get_name(), length);
        let state_holder = LengthBatchWindowStateHolder::new(
            Arc::clone(&buffer),
            Arc::clone(&expired),
            component_id.clone(),
            length,
        );

        // Register state holder with SnapshotService for persistence
        let state_holder_clone = state_holder.clone();
        let state_holder_arc: Arc<Mutex<dyn crate::core::persistence::StateHolder>> =
            Arc::new(Mutex::new(state_holder_clone));
        if let Some(snapshot_service) = app_ctx.get_snapshot_service() {
            snapshot_service.register_state_holder(component_id.clone(), state_holder_arc);
        } else {
        }

        Self {
            meta: CommonProcessorMeta::new(app_ctx, query_ctx),
            length,
            buffer,
            expired,
            state_holder: Some(state_holder),
        }
    }

    pub fn from_handler(
        handler: &WindowHandler,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
        _parse_ctx: &crate::core::util::parser::expression_parser::ExpressionParserContext,
    ) -> Result<Self, String> {
        let expr = handler
            .get_parameters()
            .first()
            .ok_or("LengthBatch window requires a parameter")?;
        if let Expression::Constant(c) = expr {
            let len = match &c.value {
                ConstantValueWithFloat::Int(i) => *i as usize,
                ConstantValueWithFloat::Long(l) => *l as usize,
                _ => return Err("LengthBatch window size must be int or long".to_string()),
            };
            Ok(Self::new(len, app_ctx, query_ctx))
        } else {
            Err("LengthBatch window size must be constant".to_string())
        }
    }

    fn flush(&self, timestamp: i64) {
        if let Some(ref next) = self.meta.next_processor {
            let expired_batch: Vec<StreamEvent> = {
                let mut ex = self.expired.lock().unwrap();
                std::mem::take(&mut *ex)
            };
            let current_batch: Vec<StreamEvent> = {
                let mut buf = self.buffer.lock().unwrap();
                std::mem::take(&mut *buf)
            };

            if expired_batch.is_empty() && current_batch.is_empty() {
                return;
            }

            // Record batch flush for incremental checkpointing
            if let Some(ref state_holder) = self.state_holder {
                state_holder.record_batch_flushed(&current_batch, &expired_batch);
            }

            let mut head: Option<Box<dyn ComplexEvent>> = None;
            let mut tail = &mut head;

            for mut e in expired_batch {
                e.set_event_type(ComplexEventType::Expired);
                e.set_timestamp(timestamp);
                *tail = Some(Box::new(e.clone_without_next()));
                tail = tail.as_mut().unwrap().mut_next_ref_option();
            }

            for e in &current_batch {
                *tail = Some(Box::new(e.clone_without_next()));
                tail = tail.as_mut().unwrap().mut_next_ref_option();
            }

            {
                let mut ex = self.expired.lock().unwrap();
                ex.extend(current_batch);
            }

            if let Some(chain) = head {
                next.lock().unwrap().process(Some(chain));
            }
        }
    }
}

impl Processor for LengthBatchWindowProcessor {
    fn process(&self, complex_event_chunk: Option<Box<dyn ComplexEvent>>) {
        if let Some(chunk) = complex_event_chunk {
            let mut current_opt = Some(chunk.as_ref() as &dyn ComplexEvent);
            while let Some(ev) = current_opt {
                if let Some(se) = ev.as_any().downcast_ref::<StreamEvent>() {
                    let se_clone = se.clone_without_next();

                    // Record state change for incremental checkpointing
                    if let Some(ref state_holder) = self.state_holder {
                        state_holder.record_event_added(&se_clone);
                    }

                    // Fix race condition: hold lock for entire check-and-modify operation
                    let last_ts = se.timestamp;
                    let should_flush = {
                        let mut buffer = self.buffer.lock().unwrap();
                        buffer.push(se_clone);
                        buffer.len() >= self.length
                    };

                    if should_flush {
                        self.flush(last_ts);
                    }
                }
                current_opt = ev.get_next();
            }
        }
    }

    fn next_processor(&self) -> Option<Arc<Mutex<dyn Processor>>> {
        self.meta.next_processor.as_ref().map(Arc::clone)
    }

    fn set_next_processor(&mut self, next: Option<Arc<Mutex<dyn Processor>>>) {
        self.meta.next_processor = next;
    }

    fn clone_processor(&self, ctx: &Arc<EventFluxQueryContext>) -> Box<dyn Processor> {
        Box::new(Self::new(
            self.length,
            Arc::clone(&self.meta.eventflux_app_context),
            Arc::clone(ctx),
        ))
    }

    fn get_eventflux_app_context(&self) -> Arc<EventFluxAppContext> {
        Arc::clone(&self.meta.eventflux_app_context)
    }
    fn get_eventflux_query_context(&self) -> Arc<EventFluxQueryContext> {
        self.meta.get_eventflux_query_context()
    }

    fn get_processing_mode(&self) -> ProcessingMode {
        ProcessingMode::BATCH
    }

    fn is_stateful(&self) -> bool {
        true
    }
}

impl WindowProcessor for LengthBatchWindowProcessor {}

impl NewStateHolder for LengthBatchWindowProcessor {
    fn schema_version(&self) -> SchemaVersion {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.schema_version()
        } else {
            SchemaVersion::new(1, 0, 0)
        }
    }

    fn serialize_state(&self, hints: &SerializationHints) -> Result<StateSnapshot, StateError> {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.serialize_state(hints)
        } else {
            Err(StateError::InvalidStateData {
                message: "No state holder available for serialization".to_string(),
            })
        }
    }

    fn deserialize_state(&self, snapshot: &StateSnapshot) -> Result<(), StateError> {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.deserialize_state(snapshot)
        } else {
            Err(StateError::InvalidStateData {
                message: "No state holder available for deserialization".to_string(),
            })
        }
    }

    fn get_changelog(&self, since: CheckpointId) -> Result<ChangeLog, StateError> {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.get_changelog(since)
        } else {
            Err(StateError::InvalidStateData {
                message: "No state holder available for changelog".to_string(),
            })
        }
    }

    fn apply_changelog(&self, changes: &ChangeLog) -> Result<(), StateError> {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.apply_changelog(changes)
        } else {
            Err(StateError::InvalidStateData {
                message: "No state holder available for changelog application".to_string(),
            })
        }
    }

    fn estimate_size(&self) -> StateSize {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.estimate_size()
        } else {
            StateSize {
                bytes: 0,
                entries: 0,
                estimated_growth_rate: 0.0,
            }
        }
    }

    fn access_pattern(&self) -> AccessPattern {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.access_pattern()
        } else {
            AccessPattern::Sequential
        }
    }

    fn component_metadata(&self) -> StateMetadata {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.component_metadata()
        } else {
            StateMetadata::new(
                "unknown_length_batch_window".to_string(),
                "LengthBatchWindowProcessor".to_string(),
            )
        }
    }
}

#[derive(Debug, Clone)]
pub struct LengthBatchWindowFactory;

impl WindowProcessorFactory for LengthBatchWindowFactory {
    fn name(&self) -> &'static str {
        "lengthBatch"
    }
    fn create(
        &self,
        handler: &WindowHandler,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
        parse_ctx: &crate::core::util::parser::expression_parser::ExpressionParserContext,
    ) -> Result<Arc<Mutex<dyn Processor>>, String> {
        Ok(Arc::new(Mutex::new(
            LengthBatchWindowProcessor::from_handler(handler, app_ctx, query_ctx, parse_ctx)?,
        )))
    }

    fn clone_box(&self) -> Box<dyn WindowProcessorFactory> {
        Box::new(Self {})
    }
}

// ---- TimeBatchWindowProcessor ----

#[derive(Debug)]
pub struct TimeBatchWindowProcessor {
    meta: CommonProcessorMeta,
    pub duration_ms: i64,
    scheduler: Option<Arc<Scheduler>>,
    buffer: Arc<Mutex<Vec<StreamEvent>>>,
    expired: Arc<Mutex<Vec<StreamEvent>>>,
    start_time: Arc<Mutex<Option<i64>>>,
    state_holder: Option<TimeBatchWindowStateHolder>,
}

impl TimeBatchWindowProcessor {
    pub fn new(
        duration_ms: i64,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
    ) -> Self {
        let scheduler = app_ctx.get_scheduler();
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let expired = Arc::new(Mutex::new(Vec::new()));
        let start_time = Arc::new(Mutex::new(None));

        // Create enhanced StateHolder
        let component_id = format!("time_batch_window_{}", uuid::Uuid::new_v4());
        let state_holder = TimeBatchWindowStateHolder::new(
            Arc::clone(&buffer),
            Arc::clone(&expired),
            Arc::clone(&start_time),
            component_id,
            duration_ms,
        );

        Self {
            meta: CommonProcessorMeta::new(app_ctx, query_ctx),
            duration_ms,
            scheduler,
            buffer,
            expired,
            start_time,
            state_holder: Some(state_holder),
        }
    }

    pub fn from_handler(
        handler: &WindowHandler,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
        _parse_ctx: &crate::core::util::parser::expression_parser::ExpressionParserContext,
    ) -> Result<Self, String> {
        let expr = handler
            .get_parameters()
            .first()
            .ok_or("TimeBatch window requires a parameter")?;
        if let Expression::Constant(c) = expr {
            let dur = match &c.value {
                ConstantValueWithFloat::Time(t) => *t,
                ConstantValueWithFloat::Long(l) => *l,
                _ => return Err("TimeBatch window duration must be time constant".to_string()),
            };
            Ok(Self::new(dur, app_ctx, query_ctx))
        } else {
            Err("TimeBatch window duration must be constant".to_string())
        }
    }

    fn flush(&self, timestamp: i64) {
        if let Some(ref next) = self.meta.next_processor {
            let expired_batch: Vec<StreamEvent> = {
                let mut ex = self.expired.lock().unwrap();
                std::mem::take(&mut *ex)
            };
            let current_batch: Vec<StreamEvent> = {
                let mut buf = self.buffer.lock().unwrap();
                std::mem::take(&mut *buf)
            };

            if expired_batch.is_empty() && current_batch.is_empty() {
                return;
            }

            // Record batch flush for incremental checkpointing
            if let Some(ref state_holder) = self.state_holder {
                state_holder.record_batch_flushed(&current_batch, &expired_batch, timestamp);
            }

            let mut head: Option<Box<dyn ComplexEvent>> = None;
            let mut tail = &mut head;

            for mut e in expired_batch {
                e.set_event_type(ComplexEventType::Expired);
                e.set_timestamp(timestamp);
                *tail = Some(Box::new(e.clone_without_next()));
                tail = tail.as_mut().unwrap().mut_next_ref_option();
            }

            for e in &current_batch {
                *tail = Some(Box::new(e.clone_without_next()));
                tail = tail.as_mut().unwrap().mut_next_ref_option();
            }

            {
                let mut ex = self.expired.lock().unwrap();
                ex.extend(current_batch);
            }

            if let Some(chain) = head {
                next.lock().unwrap().process(Some(chain));
            }
        }
        *self.start_time.lock().unwrap() = Some(timestamp);
        if let Some(ref scheduler) = self.scheduler {
            let task = BatchFlushTask {
                buffer: Arc::clone(&self.buffer),
                expired: Arc::clone(&self.expired),
                next: self.meta.next_processor.as_ref().map(Arc::clone),
                duration_ms: self.duration_ms,
                scheduler: Arc::clone(scheduler),
                start_time: Arc::clone(&self.start_time),
            };
            scheduler.notify_at(timestamp + self.duration_ms, Arc::new(task));
        }
    }
}

impl Processor for TimeBatchWindowProcessor {
    fn process(&self, complex_event_chunk: Option<Box<dyn ComplexEvent>>) {
        if let Some(chunk) = complex_event_chunk {
            let mut current_opt = Some(chunk.as_ref() as &dyn ComplexEvent);
            while let Some(ev) = current_opt {
                if let Some(se) = ev.as_any().downcast_ref::<StreamEvent>() {
                    let mut start = self.start_time.lock().unwrap();
                    let old_start_time = *start;

                    if start.is_none() {
                        *start = Some(se.timestamp);

                        // Record start time change for incremental checkpointing
                        if let Some(ref state_holder) = self.state_holder {
                            state_holder
                                .record_start_time_updated(old_start_time, Some(se.timestamp));
                        }

                        if let Some(ref scheduler) = self.scheduler {
                            let task = BatchFlushTask {
                                buffer: Arc::clone(&self.buffer),
                                expired: Arc::clone(&self.expired),
                                next: self.meta.next_processor.as_ref().map(Arc::clone),
                                duration_ms: self.duration_ms,
                                scheduler: Arc::clone(scheduler),
                                start_time: Arc::clone(&self.start_time),
                            };
                            scheduler.notify_at(se.timestamp + self.duration_ms, Arc::new(task));
                        }
                    } else if se.timestamp - start.unwrap() >= self.duration_ms {
                        let ts = se.timestamp;
                        drop(start);
                        self.flush(ts);
                    }

                    let se_clone = se.clone_without_next();

                    // Record state change for incremental checkpointing
                    if let Some(ref state_holder) = self.state_holder {
                        state_holder.record_event_added(&se_clone);
                    }

                    self.buffer.lock().unwrap().push(se_clone);
                }
                current_opt = ev.get_next();
            }
        }
    }

    fn next_processor(&self) -> Option<Arc<Mutex<dyn Processor>>> {
        self.meta.next_processor.as_ref().map(Arc::clone)
    }

    fn set_next_processor(&mut self, next: Option<Arc<Mutex<dyn Processor>>>) {
        self.meta.next_processor = next;
    }

    fn clone_processor(&self, ctx: &Arc<EventFluxQueryContext>) -> Box<dyn Processor> {
        Box::new(Self::new(
            self.duration_ms,
            Arc::clone(&self.meta.eventflux_app_context),
            Arc::clone(ctx),
        ))
    }

    fn get_eventflux_app_context(&self) -> Arc<EventFluxAppContext> {
        Arc::clone(&self.meta.eventflux_app_context)
    }
    fn get_eventflux_query_context(&self) -> Arc<EventFluxQueryContext> {
        self.meta.get_eventflux_query_context()
    }

    fn get_processing_mode(&self) -> ProcessingMode {
        ProcessingMode::BATCH
    }

    fn is_stateful(&self) -> bool {
        true
    }
}

impl WindowProcessor for TimeBatchWindowProcessor {}

impl NewStateHolder for TimeBatchWindowProcessor {
    fn schema_version(&self) -> SchemaVersion {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.schema_version()
        } else {
            SchemaVersion::new(1, 0, 0)
        }
    }

    fn serialize_state(&self, hints: &SerializationHints) -> Result<StateSnapshot, StateError> {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.serialize_state(hints)
        } else {
            Err(StateError::InvalidStateData {
                message: "No state holder available for serialization".to_string(),
            })
        }
    }

    fn deserialize_state(&self, snapshot: &StateSnapshot) -> Result<(), StateError> {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.deserialize_state(snapshot)
        } else {
            Err(StateError::InvalidStateData {
                message: "No state holder available for deserialization".to_string(),
            })
        }
    }

    fn get_changelog(&self, since: CheckpointId) -> Result<ChangeLog, StateError> {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.get_changelog(since)
        } else {
            Err(StateError::InvalidStateData {
                message: "No state holder available for changelog".to_string(),
            })
        }
    }

    fn apply_changelog(&self, changes: &ChangeLog) -> Result<(), StateError> {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.apply_changelog(changes)
        } else {
            Err(StateError::InvalidStateData {
                message: "No state holder available for changelog application".to_string(),
            })
        }
    }

    fn estimate_size(&self) -> StateSize {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.estimate_size()
        } else {
            StateSize {
                bytes: 0,
                entries: 0,
                estimated_growth_rate: 0.0,
            }
        }
    }

    fn access_pattern(&self) -> AccessPattern {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.access_pattern()
        } else {
            AccessPattern::Sequential
        }
    }

    fn component_metadata(&self) -> StateMetadata {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.component_metadata()
        } else {
            StateMetadata::new(
                "unknown_time_batch_window".to_string(),
                "TimeBatchWindowProcessor".to_string(),
            )
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimeBatchWindowFactory;

impl WindowProcessorFactory for TimeBatchWindowFactory {
    fn name(&self) -> &'static str {
        "timeBatch"
    }
    fn create(
        &self,
        handler: &WindowHandler,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
        parse_ctx: &crate::core::util::parser::expression_parser::ExpressionParserContext,
    ) -> Result<Arc<Mutex<dyn Processor>>, String> {
        Ok(Arc::new(Mutex::new(
            TimeBatchWindowProcessor::from_handler(handler, app_ctx, query_ctx, parse_ctx)?,
        )))
    }

    fn clone_box(&self) -> Box<dyn WindowProcessorFactory> {
        Box::new(Self {})
    }
}

// ---- ExternalTimeWindowProcessor ----

#[derive(Debug)]
pub struct ExternalTimeWindowProcessor {
    meta: CommonProcessorMeta,
    pub duration_ms: i64,
    buffer: Arc<Mutex<VecDeque<Arc<StreamEvent>>>>,
}

impl ExternalTimeWindowProcessor {
    pub fn new(
        duration_ms: i64,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
    ) -> Self {
        Self {
            meta: CommonProcessorMeta::new(app_ctx, query_ctx),
            duration_ms,
            buffer: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn from_handler(
        handler: &WindowHandler,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
        _parse_ctx: &crate::core::util::parser::expression_parser::ExpressionParserContext,
    ) -> Result<Self, String> {
        let expr = handler
            .get_parameters()
            .get(1)
            .ok_or("externalTime window requires a duration parameter")?;
        if let Expression::Constant(c) = expr {
            let dur = match &c.value {
                ConstantValueWithFloat::Time(t) => *t,
                ConstantValueWithFloat::Long(l) => *l,
                _ => return Err("externalTime window duration must be time constant".to_string()),
            };
            Ok(Self::new(dur, app_ctx, query_ctx))
        } else {
            Err("externalTime window duration must be constant".to_string())
        }
    }
}

impl Processor for ExternalTimeWindowProcessor {
    fn process(&self, complex_event_chunk: Option<Box<dyn ComplexEvent>>) {
        if let Some(ref next) = self.meta.next_processor {
            if let Some(chunk) = complex_event_chunk {
                let mut current_opt = Some(chunk.as_ref() as &dyn ComplexEvent);
                while let Some(ev) = current_opt {
                    if let Some(se) = ev.as_any().downcast_ref::<StreamEvent>() {
                        let ts = se.timestamp;
                        let mut expired_head: Option<Box<dyn ComplexEvent>> = None;
                        let mut tail = &mut expired_head;
                        {
                            let mut buf = self.buffer.lock().unwrap();
                            while let Some(front) = buf.front() {
                                if ts - front.timestamp >= self.duration_ms {
                                    if let Some(old) = buf.pop_front() {
                                        let mut ex = old.as_ref().clone_without_next();
                                        ex.set_event_type(ComplexEventType::Expired);
                                        ex.set_timestamp(ts);
                                        *tail = Some(Box::new(ex));
                                        tail = tail.as_mut().unwrap().mut_next_ref_option();
                                    }
                                } else {
                                    break;
                                }
                            }
                            buf.push_back(Arc::new(se.clone_without_next()));
                        }

                        *tail = Some(Box::new(se.clone_without_next()));
                        next.lock().unwrap().process(expired_head);
                    }
                    current_opt = ev.get_next();
                }
            }
        }
    }

    fn next_processor(&self) -> Option<Arc<Mutex<dyn Processor>>> {
        self.meta.next_processor.as_ref().map(Arc::clone)
    }

    fn set_next_processor(&mut self, next: Option<Arc<Mutex<dyn Processor>>>) {
        self.meta.next_processor = next;
    }

    fn clone_processor(&self, ctx: &Arc<EventFluxQueryContext>) -> Box<dyn Processor> {
        Box::new(Self::new(
            self.duration_ms,
            Arc::clone(&self.meta.eventflux_app_context),
            Arc::clone(ctx),
        ))
    }

    fn get_eventflux_app_context(&self) -> Arc<EventFluxAppContext> {
        Arc::clone(&self.meta.eventflux_app_context)
    }
    fn get_eventflux_query_context(&self) -> Arc<EventFluxQueryContext> {
        self.meta.get_eventflux_query_context()
    }

    fn get_processing_mode(&self) -> ProcessingMode {
        ProcessingMode::SLIDE
    }

    fn is_stateful(&self) -> bool {
        true
    }
}

impl WindowProcessor for ExternalTimeWindowProcessor {}

#[derive(Debug, Clone)]
pub struct ExternalTimeWindowFactory;

impl WindowProcessorFactory for ExternalTimeWindowFactory {
    fn name(&self) -> &'static str {
        "externalTime"
    }
    fn create(
        &self,
        handler: &WindowHandler,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
        parse_ctx: &crate::core::util::parser::expression_parser::ExpressionParserContext,
    ) -> Result<Arc<Mutex<dyn Processor>>, String> {
        Ok(Arc::new(Mutex::new(
            ExternalTimeWindowProcessor::from_handler(handler, app_ctx, query_ctx, parse_ctx)?,
        )))
    }

    fn clone_box(&self) -> Box<dyn WindowProcessorFactory> {
        Box::new(Self {})
    }
}

// ---- ExternalTimeBatchWindowProcessor ----

#[derive(Debug)]
pub struct ExternalTimeBatchWindowProcessor {
    meta: CommonProcessorMeta,
    pub duration_ms: i64,
    buffer: Arc<Mutex<Vec<StreamEvent>>>,
    expired: Arc<Mutex<Vec<StreamEvent>>>,
    start_time: Arc<Mutex<Option<i64>>>,
}

impl ExternalTimeBatchWindowProcessor {
    pub fn new(
        duration_ms: i64,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
    ) -> Self {
        Self {
            meta: CommonProcessorMeta::new(app_ctx, query_ctx),
            duration_ms,
            buffer: Arc::new(Mutex::new(Vec::new())),
            expired: Arc::new(Mutex::new(Vec::new())),
            start_time: Arc::new(Mutex::new(None)),
        }
    }

    pub fn from_handler(
        handler: &WindowHandler,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
        _parse_ctx: &crate::core::util::parser::expression_parser::ExpressionParserContext,
    ) -> Result<Self, String> {
        let expr = handler
            .get_parameters()
            .get(1)
            .ok_or("externalTimeBatch window requires a duration parameter")?;
        if let Expression::Constant(c) = expr {
            let dur = match &c.value {
                ConstantValueWithFloat::Time(t) => *t,
                ConstantValueWithFloat::Long(l) => *l,
                _ => {
                    return Err(
                        "externalTimeBatch window duration must be time constant".to_string()
                    )
                }
            };
            Ok(Self::new(dur, app_ctx, query_ctx))
        } else {
            Err("externalTimeBatch window duration must be constant".to_string())
        }
    }

    fn flush(&self, timestamp: i64) {
        if let Some(ref next) = self.meta.next_processor {
            let expired_batch: Vec<StreamEvent> = {
                let mut ex = self.expired.lock().unwrap();
                std::mem::take(&mut *ex)
            };
            let current_batch: Vec<StreamEvent> = {
                let mut buf = self.buffer.lock().unwrap();
                std::mem::take(&mut *buf)
            };

            if expired_batch.is_empty() && current_batch.is_empty() {
                return;
            }

            let mut head: Option<Box<dyn ComplexEvent>> = None;
            let mut tail = &mut head;

            for mut e in expired_batch {
                e.set_event_type(ComplexEventType::Expired);
                e.set_timestamp(timestamp);
                *tail = Some(Box::new(e.clone_without_next()));
                tail = tail.as_mut().unwrap().mut_next_ref_option();
            }

            for e in &current_batch {
                *tail = Some(Box::new(e.clone_without_next()));
                tail = tail.as_mut().unwrap().mut_next_ref_option();
            }

            {
                let mut ex = self.expired.lock().unwrap();
                ex.extend(current_batch);
            }

            if let Some(chain) = head {
                next.lock().unwrap().process(Some(chain));
            }
        }
        *self.start_time.lock().unwrap() = Some(timestamp);
    }
}

impl Processor for ExternalTimeBatchWindowProcessor {
    fn process(&self, complex_event_chunk: Option<Box<dyn ComplexEvent>>) {
        if let Some(chunk) = complex_event_chunk {
            let mut current_opt = Some(chunk.as_ref() as &dyn ComplexEvent);
            while let Some(ev) = current_opt {
                if let Some(se) = ev.as_any().downcast_ref::<StreamEvent>() {
                    let ts = se.timestamp;
                    let mut start = self.start_time.lock().unwrap();
                    if start.is_none() {
                        *start = Some(ts);
                    }
                    while ts - start.unwrap() >= self.duration_ms {
                        let flush_ts = start.unwrap() + self.duration_ms;
                        drop(start);
                        self.flush(flush_ts);
                        start = self.start_time.lock().unwrap();
                    }
                    self.buffer.lock().unwrap().push(se.clone_without_next());
                }
                current_opt = ev.get_next();
            }
        }
    }

    fn next_processor(&self) -> Option<Arc<Mutex<dyn Processor>>> {
        self.meta.next_processor.as_ref().map(Arc::clone)
    }

    fn set_next_processor(&mut self, next: Option<Arc<Mutex<dyn Processor>>>) {
        self.meta.next_processor = next;
    }

    fn clone_processor(&self, ctx: &Arc<EventFluxQueryContext>) -> Box<dyn Processor> {
        Box::new(Self::new(
            self.duration_ms,
            Arc::clone(&self.meta.eventflux_app_context),
            Arc::clone(ctx),
        ))
    }

    fn get_eventflux_app_context(&self) -> Arc<EventFluxAppContext> {
        Arc::clone(&self.meta.eventflux_app_context)
    }
    fn get_eventflux_query_context(&self) -> Arc<EventFluxQueryContext> {
        self.meta.get_eventflux_query_context()
    }

    fn get_processing_mode(&self) -> ProcessingMode {
        ProcessingMode::BATCH
    }

    fn is_stateful(&self) -> bool {
        true
    }
}

impl WindowProcessor for ExternalTimeBatchWindowProcessor {}

// ---- LossyCountingWindowProcessor ----

#[derive(Debug)]
pub struct LossyCountingWindowProcessor {
    meta: CommonProcessorMeta,
}

impl LossyCountingWindowProcessor {
    pub fn new(app_ctx: Arc<EventFluxAppContext>, query_ctx: Arc<EventFluxQueryContext>) -> Self {
        Self {
            meta: CommonProcessorMeta::new(app_ctx, query_ctx),
        }
    }

    pub fn from_handler(
        _handler: &WindowHandler,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
        _parse_ctx: &crate::core::util::parser::expression_parser::ExpressionParserContext,
    ) -> Result<Self, String> {
        Ok(Self::new(app_ctx, query_ctx))
    }
}

impl Processor for LossyCountingWindowProcessor {
    fn process(&self, complex_event_chunk: Option<Box<dyn ComplexEvent>>) {
        if let Some(ref next) = self.meta.next_processor {
            next.lock().unwrap().process(complex_event_chunk);
        }
    }

    fn next_processor(&self) -> Option<Arc<Mutex<dyn Processor>>> {
        self.meta.next_processor.as_ref().map(Arc::clone)
    }

    fn set_next_processor(&mut self, next: Option<Arc<Mutex<dyn Processor>>>) {
        self.meta.next_processor = next;
    }

    fn clone_processor(&self, ctx: &Arc<EventFluxQueryContext>) -> Box<dyn Processor> {
        Box::new(Self::new(
            Arc::clone(&self.meta.eventflux_app_context),
            Arc::clone(ctx),
        ))
    }

    fn get_eventflux_app_context(&self) -> Arc<EventFluxAppContext> {
        Arc::clone(&self.meta.eventflux_app_context)
    }
    fn get_eventflux_query_context(&self) -> Arc<EventFluxQueryContext> {
        self.meta.get_eventflux_query_context()
    }

    fn get_processing_mode(&self) -> ProcessingMode {
        ProcessingMode::DEFAULT
    }

    fn is_stateful(&self) -> bool {
        false
    }
}

impl WindowProcessor for LossyCountingWindowProcessor {}

#[derive(Debug, Clone)]
pub struct LossyCountingWindowFactory;

impl WindowProcessorFactory for LossyCountingWindowFactory {
    fn name(&self) -> &'static str {
        "lossyCounting"
    }
    fn create(
        &self,
        handler: &WindowHandler,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
        parse_ctx: &crate::core::util::parser::expression_parser::ExpressionParserContext,
    ) -> Result<Arc<Mutex<dyn Processor>>, String> {
        Ok(Arc::new(Mutex::new(
            LossyCountingWindowProcessor::from_handler(handler, app_ctx, query_ctx, parse_ctx)?,
        )))
    }

    fn clone_box(&self) -> Box<dyn WindowProcessorFactory> {
        Box::new(Self {})
    }
}

// ---- CronWindowProcessor ----

#[derive(Debug)]
pub struct CronWindowProcessor {
    meta: CommonProcessorMeta,
    cron: String,
    scheduler: Option<Arc<Scheduler>>,
    buffer: Arc<Mutex<Vec<StreamEvent>>>,
    expired: Arc<Mutex<Vec<StreamEvent>>>,
    scheduled: Arc<Mutex<bool>>,
}

impl CronWindowProcessor {
    pub fn new(
        cron: String,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
    ) -> Self {
        let scheduler = app_ctx.get_scheduler();
        Self {
            meta: CommonProcessorMeta::new(app_ctx, query_ctx),
            cron,
            scheduler,
            buffer: Arc::new(Mutex::new(Vec::new())),
            expired: Arc::new(Mutex::new(Vec::new())),
            scheduled: Arc::new(Mutex::new(false)),
        }
    }

    pub fn from_handler(
        handler: &WindowHandler,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
        _parse_ctx: &crate::core::util::parser::expression_parser::ExpressionParserContext,
    ) -> Result<Self, String> {
        let expr = handler
            .get_parameters()
            .first()
            .ok_or("cron window requires a cron expression")?;
        if let Expression::Constant(c) = expr {
            if let ConstantValueWithFloat::String(s) = &c.value {
                Ok(Self::new(s.clone(), app_ctx, query_ctx))
            } else {
                Err("cron expression must be a string".to_string())
            }
        } else {
            Err("cron expression must be constant".to_string())
        }
    }

    fn schedule(&self) {
        if let Some(ref sched) = self.scheduler {
            if !*self.scheduled.lock().unwrap() {
                let task = CronFlushTask {
                    buffer: Arc::clone(&self.buffer),
                    expired: Arc::clone(&self.expired),
                    next: self.meta.next_processor.as_ref().map(Arc::clone),
                };
                let _ = sched.schedule_cron(&self.cron, Arc::new(task), None);
                *self.scheduled.lock().unwrap() = true;
            }
        }
    }
}

#[derive(Clone)]
struct CronFlushTask {
    buffer: Arc<Mutex<Vec<StreamEvent>>>,
    expired: Arc<Mutex<Vec<StreamEvent>>>,
    next: Option<Arc<Mutex<dyn Processor>>>,
}

impl Schedulable for CronFlushTask {
    fn on_time(&self, timestamp: i64) {
        let expired_batch: Vec<StreamEvent> = {
            let mut ex = self.expired.lock().unwrap();
            std::mem::take(&mut *ex)
        };
        let current_batch: Vec<StreamEvent> = {
            let mut buf = self.buffer.lock().unwrap();
            std::mem::take(&mut *buf)
        };

        if expired_batch.is_empty() && current_batch.is_empty() {
            return;
        }

        let mut head: Option<Box<dyn ComplexEvent>> = None;
        let mut tail = &mut head;

        for mut e in expired_batch {
            e.set_event_type(ComplexEventType::Expired);
            e.set_timestamp(timestamp);
            *tail = Some(Box::new(e.clone_without_next()));
            tail = tail.as_mut().unwrap().mut_next_ref_option();
        }

        for e in &current_batch {
            *tail = Some(Box::new(e.clone_without_next()));
            tail = tail.as_mut().unwrap().mut_next_ref_option();
        }

        {
            let mut ex = self.expired.lock().unwrap();
            ex.extend(current_batch);
        }

        if let Some(chain) = head {
            if let Some(ref next) = self.next {
                next.lock().unwrap().process(Some(chain));
            }
        }
    }
}

impl Processor for CronWindowProcessor {
    fn process(&self, complex_event_chunk: Option<Box<dyn ComplexEvent>>) {
        if let Some(chunk) = complex_event_chunk {
            let mut current_opt = Some(chunk.as_ref() as &dyn ComplexEvent);
            while let Some(ev) = current_opt {
                if let Some(se) = ev.as_any().downcast_ref::<StreamEvent>() {
                    self.buffer.lock().unwrap().push(se.clone_without_next());
                }
                current_opt = ev.get_next();
            }
        }
    }

    fn next_processor(&self) -> Option<Arc<Mutex<dyn Processor>>> {
        self.meta.next_processor.as_ref().map(Arc::clone)
    }

    fn set_next_processor(&mut self, next: Option<Arc<Mutex<dyn Processor>>>) {
        self.meta.next_processor = next;
        self.schedule();
    }

    fn clone_processor(&self, ctx: &Arc<EventFluxQueryContext>) -> Box<dyn Processor> {
        Box::new(Self::new(
            self.cron.clone(),
            Arc::clone(&self.meta.eventflux_app_context),
            Arc::clone(ctx),
        ))
    }

    fn get_eventflux_app_context(&self) -> Arc<EventFluxAppContext> {
        Arc::clone(&self.meta.eventflux_app_context)
    }
    fn get_eventflux_query_context(&self) -> Arc<EventFluxQueryContext> {
        self.meta.get_eventflux_query_context()
    }

    fn get_processing_mode(&self) -> ProcessingMode {
        ProcessingMode::BATCH
    }

    fn is_stateful(&self) -> bool {
        true
    }
}

impl WindowProcessor for CronWindowProcessor {}

#[derive(Debug, Clone)]
pub struct CronWindowFactory;

impl WindowProcessorFactory for CronWindowFactory {
    fn name(&self) -> &'static str {
        "cron"
    }
    fn create(
        &self,
        handler: &WindowHandler,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
        parse_ctx: &crate::core::util::parser::expression_parser::ExpressionParserContext,
    ) -> Result<Arc<Mutex<dyn Processor>>, String> {
        Ok(Arc::new(Mutex::new(CronWindowProcessor::from_handler(
            handler, app_ctx, query_ctx, parse_ctx,
        )?)))
    }

    fn clone_box(&self) -> Box<dyn WindowProcessorFactory> {
        Box::new(Self {})
    }
}

#[derive(Debug, Clone)]
pub struct ExternalTimeBatchWindowFactory;

impl WindowProcessorFactory for ExternalTimeBatchWindowFactory {
    fn name(&self) -> &'static str {
        "externalTimeBatch"
    }
    fn create(
        &self,
        handler: &WindowHandler,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
        parse_ctx: &crate::core::util::parser::expression_parser::ExpressionParserContext,
    ) -> Result<Arc<Mutex<dyn Processor>>, String> {
        Ok(Arc::new(Mutex::new(
            ExternalTimeBatchWindowProcessor::from_handler(handler, app_ctx, query_ctx, parse_ctx)?,
        )))
    }

    fn clone_box(&self) -> Box<dyn WindowProcessorFactory> {
        Box::new(Self {})
    }
}

#[derive(Debug, Clone)]
pub struct SessionWindowFactory;

impl WindowProcessorFactory for SessionWindowFactory {
    fn name(&self) -> &'static str {
        "session"
    }
    fn create(
        &self,
        handler: &WindowHandler,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
        parse_ctx: &crate::core::util::parser::expression_parser::ExpressionParserContext,
    ) -> Result<Arc<Mutex<dyn Processor>>, String> {
        Ok(Arc::new(Mutex::new(SessionWindowProcessor::from_handler(
            handler, app_ctx, query_ctx, parse_ctx,
        )?)))
    }

    fn clone_box(&self) -> Box<dyn WindowProcessorFactory> {
        Box::new(Self {})
    }
}

#[derive(Debug, Clone)]
pub struct SortWindowFactory;

impl WindowProcessorFactory for SortWindowFactory {
    fn name(&self) -> &'static str {
        "sort"
    }

    fn create(
        &self,
        handler: &WindowHandler,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
        parse_ctx: &crate::core::util::parser::expression_parser::ExpressionParserContext,
    ) -> Result<Arc<Mutex<dyn Processor>>, String> {
        Ok(Arc::new(Mutex::new(SortWindowProcessor::from_handler(
            handler, app_ctx, query_ctx, parse_ctx,
        )?)))
    }

    fn clone_box(&self) -> Box<dyn WindowProcessorFactory> {
        Box::new(Self {})
    }
}
