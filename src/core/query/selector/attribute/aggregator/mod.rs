// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use crate::core::config::eventflux_app_context::EventFluxAppContext;
use crate::core::config::eventflux_query_context::EventFluxQueryContext;
use crate::core::event::complex_event::{ComplexEvent, ComplexEventType};
use crate::core::event::value::AttributeValue;
use crate::core::executor::expression_executor::ExpressionExecutor;
use crate::core::query::processor::ProcessingMode;
use crate::query_api::definition::attribute::Type as ApiAttributeType;

pub trait AttributeAggregatorExecutor: ExpressionExecutor {
    fn init(
        &mut self,
        executors: Vec<Box<dyn ExpressionExecutor>>,
        processing_mode: ProcessingMode,
        expired_output: bool,
        ctx: &EventFluxQueryContext,
    ) -> Result<(), String>;

    fn process_add(&self, data: Option<AttributeValue>) -> Option<AttributeValue>;
    fn process_remove(&self, data: Option<AttributeValue>) -> Option<AttributeValue>;
    fn reset(&self) -> Option<AttributeValue>;
    fn clone_box(&self) -> Box<dyn AttributeAggregatorExecutor>;
}

impl Clone for Box<dyn AttributeAggregatorExecutor> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Adapter that allows returning attribute aggregators as `Box<dyn ExpressionExecutor>`
/// on stable Rust, without relying on unstable trait upcasting coercions.
pub struct AttributeAggregatorExpressionExecutor {
    inner: Box<dyn AttributeAggregatorExecutor>,
}

impl std::fmt::Debug for AttributeAggregatorExpressionExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AttributeAggregatorExpressionExecutor")
            .finish_non_exhaustive()
    }
}

impl AttributeAggregatorExpressionExecutor {
    pub fn new(inner: Box<dyn AttributeAggregatorExecutor>) -> Self {
        Self { inner }
    }
}

impl ExpressionExecutor for AttributeAggregatorExpressionExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        self.inner.execute(event)
    }

    fn get_return_type(&self) -> ApiAttributeType {
        self.inner.get_return_type()
    }

    fn clone_executor(
        &self,
        _eventflux_app_context: &Arc<EventFluxAppContext>,
    ) -> Box<dyn ExpressionExecutor> {
        Box::new(Self::new(self.inner.clone_box()))
    }

    fn is_attribute_aggregator(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod adapter_tests {
    use super::*;
    use crate::core::config::eventflux_context::EventFluxContext;
    use crate::query_api::EventFluxApp;

    #[derive(Debug, Clone)]
    struct TestAggregator;

    impl ExpressionExecutor for TestAggregator {
        fn execute(&self, _event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
            None
        }

        fn get_return_type(&self) -> ApiAttributeType {
            ApiAttributeType::INT
        }

        fn clone_executor(
            &self,
            _eventflux_app_context: &Arc<EventFluxAppContext>,
        ) -> Box<dyn ExpressionExecutor> {
            Box::new(self.clone())
        }

        fn is_attribute_aggregator(&self) -> bool {
            true
        }
    }

    impl AttributeAggregatorExecutor for TestAggregator {
        fn init(
            &mut self,
            _executors: Vec<Box<dyn ExpressionExecutor>>,
            _processing_mode: ProcessingMode,
            _expired_output: bool,
            _ctx: &EventFluxQueryContext,
        ) -> Result<(), String> {
            Ok(())
        }

        fn process_add(&self, _data: Option<AttributeValue>) -> Option<AttributeValue> {
            None
        }

        fn process_remove(&self, _data: Option<AttributeValue>) -> Option<AttributeValue> {
            None
        }

        fn reset(&self) -> Option<AttributeValue> {
            None
        }

        fn clone_box(&self) -> Box<dyn AttributeAggregatorExecutor> {
            Box::new(self.clone())
        }
    }

    #[test]
    fn adapter_clones_as_expression_executor_and_preserves_flag() {
        let ctx = Arc::new(EventFluxAppContext::new(
            Arc::new(EventFluxContext::default()),
            "test".to_string(),
            Arc::new(EventFluxApp::default()),
            "".to_string(),
        ));

        let adapter = AttributeAggregatorExpressionExecutor::new(Box::new(TestAggregator));
        assert!(adapter.is_attribute_aggregator());
        assert_eq!(adapter.get_return_type(), ApiAttributeType::INT);

        let cloned = adapter.clone_executor(&ctx);
        assert!(cloned.is_attribute_aggregator());
        assert_eq!(cloned.get_return_type(), ApiAttributeType::INT);

        let _ = format!("{cloned:?}");
    }
}

fn value_as_f64(v: &AttributeValue) -> Option<f64> {
    match v {
        AttributeValue::Int(i) => Some(*i as f64),
        AttributeValue::Long(l) => Some(*l as f64),
        AttributeValue::Float(f) => Some(*f as f64),
        AttributeValue::Double(d) => Some(*d),
        _ => None,
    }
}

#[derive(Debug, Clone, Default)]
struct SumState {
    sum: f64,
    count: u64,
}

#[derive(Debug)]
pub struct SumAttributeAggregatorExecutor {
    arg_exec: Option<Box<dyn ExpressionExecutor>>,
    return_type: ApiAttributeType,
    state: Mutex<SumState>,
    app_ctx: Option<Arc<EventFluxAppContext>>,
    state_holder: Option<SumAggregatorStateHolder>,
    // Shared state for persistence (same as used by state holder)
    shared_sum: Option<Arc<Mutex<f64>>>,
    shared_count: Option<Arc<Mutex<u64>>>,
}

impl Default for SumAttributeAggregatorExecutor {
    fn default() -> Self {
        Self {
            arg_exec: None,
            return_type: ApiAttributeType::DOUBLE,
            state: Mutex::new(SumState::default()),
            app_ctx: None,
            state_holder: None,
            shared_sum: None,
            shared_count: None,
        }
    }
}

impl AttributeAggregatorExecutor for SumAttributeAggregatorExecutor {
    fn init(
        &mut self,
        mut executors: Vec<Box<dyn ExpressionExecutor>>,
        _mode: ProcessingMode,
        _expired_output: bool,
        ctx: &EventFluxQueryContext,
    ) -> Result<(), String> {
        if executors.len() != 1 {
            return Err("sum() requires exactly one argument".to_string());
        }
        let exec = executors.remove(0);
        let rtype = match exec.get_return_type() {
            ApiAttributeType::INT | ApiAttributeType::LONG => ApiAttributeType::LONG,
            ApiAttributeType::FLOAT | ApiAttributeType::DOUBLE => ApiAttributeType::DOUBLE,
            t => return Err(format!("sum not supported for {t:?}")),
        };
        self.return_type = rtype;
        self.arg_exec = Some(exec);
        self.app_ctx = Some(Arc::clone(&ctx.eventflux_app_context));

        // Initialize state holder for enterprise state management
        let sum_arc = Arc::new(Mutex::new(0.0f64));
        let count_arc = Arc::new(Mutex::new(0u64));
        // Use deterministic component ID with unique aggregator index for state recovery compatibility
        let aggregator_id = ctx.next_aggregator_id();
        let component_id = format!("sum_aggregator_{}", aggregator_id);

        let state_holder = SumAggregatorStateHolder::new(
            sum_arc.clone(),
            count_arc.clone(),
            component_id.clone(),
            rtype,
        );

        // Register state holder with SnapshotService for persistence
        let state_holder_arc: Arc<Mutex<dyn crate::core::persistence::StateHolder>> =
            Arc::new(Mutex::new(state_holder.clone()));
        ctx.register_state_holder(component_id, state_holder_arc);

        // Store shared state references for synchronized updates
        self.shared_sum = Some(sum_arc);
        self.shared_count = Some(count_arc);
        self.state_holder = Some(state_holder);

        Ok(())
    }

    fn process_add(&self, data: Option<AttributeValue>) -> Option<AttributeValue> {
        if let Some(v) = data.as_ref().and_then(value_as_f64) {
            // Update internal state with sync check for post-restoration scenarios
            let mut st = self.state.lock().unwrap();

            // Sync internal state with shared state before processing (only for non-group aggregators)
            if let (Some(ref shared_sum), Some(ref shared_count)) =
                (&self.shared_sum, &self.shared_count)
            {
                let shared_sum_val = *shared_sum.lock().unwrap();
                let shared_count_val = *shared_count.lock().unwrap();

                // Only sync if there's a significant difference (indicating restoration happened)
                if (st.sum - shared_sum_val).abs() > 0.0001 || st.count != shared_count_val {
                    st.sum = shared_sum_val;
                    st.count = shared_count_val;
                }
            }

            st.sum += v;
            st.count += 1;

            // Update shared state for persistence (only for non-group aggregators)
            if let (Some(ref shared_sum), Some(ref shared_count)) =
                (&self.shared_sum, &self.shared_count)
            {
                *shared_sum.lock().unwrap() = st.sum;
                *shared_count.lock().unwrap() = st.count;
            }

            // Record state change for incremental checkpointing
            if let Some(ref state_holder) = self.state_holder {
                state_holder.record_value_added(v);
            }
        }
        let st = self.state.lock().unwrap();
        let result = match self.return_type {
            ApiAttributeType::LONG => Some(AttributeValue::Long(st.sum as i64)),
            ApiAttributeType::DOUBLE => Some(AttributeValue::Double(st.sum)),
            _ => None,
        };
        result
    }

    fn process_remove(&self, data: Option<AttributeValue>) -> Option<AttributeValue> {
        if let Some(v) = data.as_ref().and_then(value_as_f64) {
            // Update internal state with sync check for post-restoration scenarios
            let mut st = self.state.lock().unwrap();

            // Sync internal state with shared state before processing (only for non-group aggregators)
            if let (Some(ref shared_sum), Some(ref shared_count)) =
                (&self.shared_sum, &self.shared_count)
            {
                let shared_sum_val = *shared_sum.lock().unwrap();
                let shared_count_val = *shared_count.lock().unwrap();

                // Only sync if there's a significant difference (indicating restoration happened)
                if (st.sum - shared_sum_val).abs() > 0.0001 || st.count != shared_count_val {
                    st.sum = shared_sum_val;
                    st.count = shared_count_val;
                }
            }

            st.sum -= v;
            if st.count > 0 {
                st.count -= 1;
            }

            // Update shared state for persistence (only for non-group aggregators)
            if let (Some(ref shared_sum), Some(ref shared_count)) =
                (&self.shared_sum, &self.shared_count)
            {
                *shared_sum.lock().unwrap() = st.sum;
                *shared_count.lock().unwrap() = st.count;
            }

            // Record state change for incremental checkpointing
            if let Some(ref state_holder) = self.state_holder {
                state_holder.record_value_removed(v);
            }
        }
        let st = self.state.lock().unwrap();
        match self.return_type {
            ApiAttributeType::LONG => Some(AttributeValue::Long(st.sum as i64)),
            ApiAttributeType::DOUBLE => Some(AttributeValue::Double(st.sum)),
            _ => None,
        }
    }

    fn reset(&self) -> Option<AttributeValue> {
        // Update internal state
        let mut st = self.state.lock().unwrap();
        let old_sum = st.sum;
        let old_count = st.count;

        st.sum = 0.0;
        st.count = 0;

        // Update shared state for persistence
        if let (Some(ref shared_sum), Some(ref shared_count)) =
            (&self.shared_sum, &self.shared_count)
        {
            *shared_sum.lock().unwrap() = 0.0;
            *shared_count.lock().unwrap() = 0;
        }

        // Record state reset for incremental checkpointing
        if let Some(ref state_holder) = self.state_holder {
            state_holder.record_reset(old_sum, old_count);
        }

        None
    }

    fn clone_box(&self) -> Box<dyn AttributeAggregatorExecutor> {
        let ctx = self.app_ctx.as_ref().unwrap();

        // Always sync internal state with shared state before cloning to ensure consistency
        let synchronized_state = if let (Some(shared_sum), Some(shared_count)) =
            (&self.shared_sum, &self.shared_count)
        {
            let shared_sum_val = *shared_sum.lock().unwrap();
            let shared_count_val = *shared_count.lock().unwrap();
            SumState {
                sum: shared_sum_val,
                count: shared_count_val,
            }
        } else {
            let current_state = self.state.lock().unwrap().clone();
            current_state
        };

        // Create independent shared state for each group to prevent cross-group interference
        let group_shared_sum = Arc::new(Mutex::new(synchronized_state.sum));
        let group_shared_count = Arc::new(Mutex::new(synchronized_state.count));

        Box::new(SumAttributeAggregatorExecutor {
            arg_exec: self.arg_exec.as_ref().map(|e| e.clone_executor(ctx)),
            return_type: self.return_type,
            state: Mutex::new(synchronized_state),
            app_ctx: Some(Arc::clone(ctx)),
            state_holder: None, // Group aggregators don't need individual state holders
            shared_sum: Some(group_shared_sum), // Independent shared state per group
            shared_count: Some(group_shared_count), // Independent shared state per group
        })
    }
}

impl ExpressionExecutor for SumAttributeAggregatorExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let event = event?;
        let data = self.arg_exec.as_ref().and_then(|e| e.execute(Some(event)));
        match event.get_event_type() {
            ComplexEventType::Current => self.process_add(data),
            ComplexEventType::Expired => self.process_remove(data),
            ComplexEventType::Reset => self.reset(),
            _ => None,
        }
    }

    fn get_return_type(&self) -> ApiAttributeType {
        self.return_type
    }

    fn clone_executor(&self, _ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(AttributeAggregatorExpressionExecutor::new(self.clone_box()))
    }

    fn is_attribute_aggregator(&self) -> bool {
        true
    }
}

// StateHolder implementation for SumAttributeAggregatorExecutor
impl crate::core::persistence::state_holder::StateHolder for SumAttributeAggregatorExecutor {
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
                crate::core::persistence::state_holder::StateError::SerializationError {
                    message: "No state holder available".to_string(),
                },
            )
        }
    }

    fn deserialize_state(
        &self,
        snapshot: &crate::core::persistence::state_holder::StateSnapshot,
    ) -> Result<(), crate::core::persistence::state_holder::StateError> {
        if let Some(ref state_holder) = self.state_holder {
            // First restore the state holder
            let result = state_holder.deserialize_state(snapshot);

            // Then synchronize the executor's internal state with the restored state holder
            if result.is_ok() {
                let restored_sum = state_holder.get_sum();
                let restored_count = state_holder.get_count();
                let mut st = self.state.lock().unwrap();
                st.sum = restored_sum;
                st.count = restored_count;

                // Also synchronize shared state if available
                if let Some(ref shared_sum) = self.shared_sum {
                    if let Ok(mut shared) = shared_sum.try_lock() {
                        *shared = restored_sum;
                    }
                }
                if let Some(ref shared_count) = self.shared_count {
                    if let Ok(mut shared) = shared_count.try_lock() {
                        *shared = restored_count;
                    }
                }
            }

            result
        } else {
            Err(
                crate::core::persistence::state_holder::StateError::DeserializationError {
                    message: "No state holder available".to_string(),
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
                crate::core::persistence::state_holder::StateError::CheckpointNotFound {
                    checkpoint_id: since,
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
                crate::core::persistence::state_holder::StateError::DeserializationError {
                    message: "No state holder available".to_string(),
                },
            )
        }
    }

    fn estimate_size(&self) -> crate::core::persistence::state_holder::StateSize {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.estimate_size()
        } else {
            crate::core::persistence::state_holder::StateSize {
                bytes: 16, // f64 + u64
                entries: 1,
                estimated_growth_rate: 0.0,
            }
        }
    }

    fn access_pattern(&self) -> crate::core::persistence::state_holder::AccessPattern {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.access_pattern()
        } else {
            crate::core::persistence::state_holder::AccessPattern::Random
        }
    }

    fn component_metadata(&self) -> crate::core::persistence::state_holder::StateMetadata {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.component_metadata()
        } else {
            crate::core::persistence::state_holder::StateMetadata::new(
                "unknown_sum_aggregator".to_string(),
                "SumAttributeAggregatorExecutor".to_string(),
            )
        }
    }
}

#[derive(Debug, Clone, Default)]
struct AvgState {
    sum: f64,
    count: u64,
}

#[derive(Debug)]
pub struct AvgAttributeAggregatorExecutor {
    arg_exec: Option<Box<dyn ExpressionExecutor>>,
    state: Mutex<AvgState>,
    app_ctx: Option<Arc<EventFluxAppContext>>,
    state_holder: Option<AvgAggregatorStateHolder>,
}

impl Default for AvgAttributeAggregatorExecutor {
    fn default() -> Self {
        Self {
            arg_exec: None,
            state: Mutex::new(AvgState::default()),
            app_ctx: None,
            state_holder: None,
        }
    }
}

impl AttributeAggregatorExecutor for AvgAttributeAggregatorExecutor {
    fn init(
        &mut self,
        mut execs: Vec<Box<dyn ExpressionExecutor>>,
        _m: ProcessingMode,
        _e: bool,
        ctx: &EventFluxQueryContext,
    ) -> Result<(), String> {
        if execs.len() != 1 {
            return Err("avg() requires one argument".to_string());
        }
        self.arg_exec = Some(execs.remove(0));
        self.app_ctx = Some(Arc::clone(&ctx.eventflux_app_context));

        // Initialize state holder for enterprise state management
        let sum_arc = Arc::new(Mutex::new(0.0f64));
        let count_arc = Arc::new(Mutex::new(0u64));
        let aggregator_id = ctx.next_aggregator_id();
        let component_id = format!("avg_aggregator_{}", aggregator_id);

        let state_holder = AvgAggregatorStateHolder::new(sum_arc, count_arc, component_id.clone());

        // Register state holder with SnapshotService for persistence
        let state_holder_arc: Arc<Mutex<dyn crate::core::persistence::StateHolder>> =
            Arc::new(Mutex::new(state_holder.clone()));
        ctx.register_state_holder(component_id, state_holder_arc);

        self.state_holder = Some(state_holder);

        Ok(())
    }

    fn process_add(&self, data: Option<AttributeValue>) -> Option<AttributeValue> {
        if let Some(v) = data.as_ref().and_then(value_as_f64) {
            let mut st = self.state.lock().unwrap();
            st.sum += v;
            st.count += 1;

            // Record state change for incremental checkpointing
            if let Some(ref state_holder) = self.state_holder {
                state_holder.record_value_added(v);
            }
        }
        let st = self.state.lock().unwrap();
        if st.count == 0 {
            None
        } else {
            Some(AttributeValue::Double(st.sum / st.count as f64))
        }
    }

    fn process_remove(&self, data: Option<AttributeValue>) -> Option<AttributeValue> {
        if let Some(v) = data.as_ref().and_then(value_as_f64) {
            let mut st = self.state.lock().unwrap();
            st.sum -= v;
            if st.count > 0 {
                st.count -= 1;
            }

            // Record state change for incremental checkpointing
            if let Some(ref state_holder) = self.state_holder {
                state_holder.record_value_removed(v);
            }
        }
        let st = self.state.lock().unwrap();
        if st.count == 0 {
            None
        } else {
            Some(AttributeValue::Double(st.sum / st.count as f64))
        }
    }

    fn reset(&self) -> Option<AttributeValue> {
        let mut st = self.state.lock().unwrap();
        let old_sum = st.sum;
        let old_count = st.count;

        st.sum = 0.0;
        st.count = 0;

        // Record state reset for incremental checkpointing
        if let Some(ref state_holder) = self.state_holder {
            state_holder.record_reset(old_sum, old_count);
        }

        None
    }

    fn clone_box(&self) -> Box<dyn AttributeAggregatorExecutor> {
        let ctx = self.app_ctx.as_ref().unwrap();
        Box::new(AvgAttributeAggregatorExecutor {
            arg_exec: self.arg_exec.as_ref().map(|e| e.clone_executor(ctx)),
            state: Mutex::new(self.state.lock().unwrap().clone()),
            app_ctx: Some(Arc::clone(ctx)),
            state_holder: self.state_holder.clone(),
        })
    }
}

impl ExpressionExecutor for AvgAttributeAggregatorExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let event = event?;
        let data = self.arg_exec.as_ref().and_then(|e| e.execute(Some(event)));
        match event.get_event_type() {
            ComplexEventType::Current => self.process_add(data),
            ComplexEventType::Expired => self.process_remove(data),
            ComplexEventType::Reset => self.reset(),
            _ => None,
        }
    }

    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::DOUBLE
    }

    fn clone_executor(&self, _ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(AttributeAggregatorExpressionExecutor::new(self.clone_box()))
    }

    fn is_attribute_aggregator(&self) -> bool {
        true
    }
}

// StateHolder implementation for AvgAttributeAggregatorExecutor
impl crate::core::persistence::state_holder::StateHolder for AvgAttributeAggregatorExecutor {
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
                crate::core::persistence::state_holder::StateError::SerializationError {
                    message: "No state holder available".to_string(),
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
                crate::core::persistence::state_holder::StateError::DeserializationError {
                    message: "No state holder available".to_string(),
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
                crate::core::persistence::state_holder::StateError::CheckpointNotFound {
                    checkpoint_id: since,
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
                crate::core::persistence::state_holder::StateError::DeserializationError {
                    message: "No state holder available".to_string(),
                },
            )
        }
    }

    fn estimate_size(&self) -> crate::core::persistence::state_holder::StateSize {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.estimate_size()
        } else {
            crate::core::persistence::state_holder::StateSize {
                bytes: 16, // f64 + u64
                entries: 1,
                estimated_growth_rate: 0.0,
            }
        }
    }

    fn access_pattern(&self) -> crate::core::persistence::state_holder::AccessPattern {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.access_pattern()
        } else {
            crate::core::persistence::state_holder::AccessPattern::Random
        }
    }

    fn component_metadata(&self) -> crate::core::persistence::state_holder::StateMetadata {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.component_metadata()
        } else {
            crate::core::persistence::state_holder::StateMetadata::new(
                "unknown_avg_aggregator".to_string(),
                "AvgAttributeAggregatorExecutor".to_string(),
            )
        }
    }
}

#[derive(Debug, Clone, Default)]
struct CountState {
    count: i64,
}

#[derive(Debug)]
pub struct CountAttributeAggregatorExecutor {
    state: Mutex<CountState>,
    app_ctx: Option<Arc<EventFluxAppContext>>,
    state_holder: Option<CountAggregatorStateHolder>,
    // Shared state for persistence (same as used by state holder)
    shared_count: Option<Arc<Mutex<i64>>>,
}

impl Default for CountAttributeAggregatorExecutor {
    fn default() -> Self {
        Self {
            state: Mutex::new(CountState::default()),
            app_ctx: None,
            state_holder: None,
            shared_count: None,
        }
    }
}

impl AttributeAggregatorExecutor for CountAttributeAggregatorExecutor {
    fn init(
        &mut self,
        _e: Vec<Box<dyn ExpressionExecutor>>,
        _m: ProcessingMode,
        _ex: bool,
        ctx: &EventFluxQueryContext,
    ) -> Result<(), String> {
        self.app_ctx = Some(Arc::clone(&ctx.eventflux_app_context));

        // Initialize state holder for enterprise state management
        let count_arc = Arc::new(Mutex::new(0i64));
        let aggregator_id = ctx.next_aggregator_id();
        let component_id = format!("count_aggregator_{}", aggregator_id);

        let state_holder = CountAggregatorStateHolder::new(count_arc.clone(), component_id.clone());

        // Register state holder with SnapshotService for persistence
        let state_holder_arc: Arc<Mutex<dyn crate::core::persistence::StateHolder>> =
            Arc::new(Mutex::new(state_holder.clone()));
        ctx.register_state_holder(component_id, state_holder_arc);

        // Store shared state reference for synchronized updates
        self.shared_count = Some(count_arc);
        self.state_holder = Some(state_holder);

        Ok(())
    }

    fn process_add(&self, _d: Option<AttributeValue>) -> Option<AttributeValue> {
        let mut st = self.state.lock().unwrap();

        // Sync internal state with shared state before processing (for post-restoration scenarios)
        if let Some(ref shared_count) = self.shared_count {
            let shared_count_val = *shared_count.lock().unwrap();

            // Only sync if there's a significant difference (indicating restoration happened)
            if st.count != shared_count_val {
                st.count = shared_count_val;
            }
        }

        st.count += 1;
        let new_count = st.count;

        // Synchronize with shared state for persistence
        if let Some(ref shared_count) = self.shared_count {
            if let Ok(mut shared) = shared_count.try_lock() {
                *shared = new_count;
            }
        }

        // Record state change for incremental checkpointing
        if let Some(ref state_holder) = self.state_holder {
            state_holder.record_increment();
        }

        Some(AttributeValue::Long(new_count))
    }
    fn process_remove(&self, _d: Option<AttributeValue>) -> Option<AttributeValue> {
        let mut st = self.state.lock().unwrap();

        // Sync internal state with shared state before processing (for post-restoration scenarios)
        if let Some(ref shared_count) = self.shared_count {
            let shared_count_val = *shared_count.lock().unwrap();

            // Only sync if there's a significant difference (indicating restoration happened)
            if st.count != shared_count_val {
                st.count = shared_count_val;
            }
        }

        st.count -= 1;
        let new_count = st.count;

        // Synchronize with shared state for persistence
        if let Some(ref shared_count) = self.shared_count {
            if let Ok(mut shared) = shared_count.try_lock() {
                *shared = new_count;
            }
        }

        // Record state change for incremental checkpointing
        if let Some(ref state_holder) = self.state_holder {
            state_holder.record_decrement();
        }

        Some(AttributeValue::Long(new_count))
    }
    fn reset(&self) -> Option<AttributeValue> {
        let mut st = self.state.lock().unwrap();
        let old_count = st.count;
        st.count = 0;

        // Synchronize with shared state for persistence
        if let Some(ref shared_count) = self.shared_count {
            if let Ok(mut shared) = shared_count.try_lock() {
                *shared = 0;
            }
        }

        // Record state reset for incremental checkpointing
        if let Some(ref state_holder) = self.state_holder {
            state_holder.record_reset(old_count);
        }

        Some(AttributeValue::Long(0))
    }
    fn clone_box(&self) -> Box<dyn AttributeAggregatorExecutor> {
        // Sync internal state with shared state before cloning
        let synchronized_state = if let Some(shared_count) = &self.shared_count {
            CountState {
                count: *shared_count.lock().unwrap(),
            }
        } else {
            self.state.lock().unwrap().clone()
        };

        // Create independent shared state for each group to prevent cross-group interference
        let group_shared_count = Arc::new(Mutex::new(synchronized_state.count));

        Box::new(CountAttributeAggregatorExecutor {
            state: Mutex::new(synchronized_state),
            app_ctx: self.app_ctx.as_ref().cloned(),
            state_holder: None, // Group aggregators don't need individual state holders
            shared_count: Some(group_shared_count), // Independent shared state per group
        })
    }
}

impl ExpressionExecutor for CountAttributeAggregatorExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let event = event?;
        match event.get_event_type() {
            ComplexEventType::Current => self.process_add(None),
            ComplexEventType::Expired => self.process_remove(None),
            ComplexEventType::Reset => self.reset(),
            _ => None,
        }
    }

    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::LONG
    }

    fn clone_executor(&self, _ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(AttributeAggregatorExpressionExecutor::new(self.clone_box()))
    }

    fn is_attribute_aggregator(&self) -> bool {
        true
    }
}

// StateHolder implementation for CountAttributeAggregatorExecutor
impl crate::core::persistence::state_holder::StateHolder for CountAttributeAggregatorExecutor {
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
                crate::core::persistence::state_holder::StateError::SerializationError {
                    message: "No state holder available".to_string(),
                },
            )
        }
    }

    fn deserialize_state(
        &self,
        snapshot: &crate::core::persistence::state_holder::StateSnapshot,
    ) -> Result<(), crate::core::persistence::state_holder::StateError> {
        if let Some(ref state_holder) = self.state_holder {
            // First restore the state holder
            let result = state_holder.deserialize_state(snapshot);

            // Then synchronize the executor's internal state with the restored state holder
            if result.is_ok() {
                let restored_count = state_holder.get_count();
                let mut st = self.state.lock().unwrap();
                st.count = restored_count;

                // Also synchronize shared state if available
                if let Some(ref shared_count) = self.shared_count {
                    if let Ok(mut shared) = shared_count.try_lock() {
                        *shared = restored_count;
                    }
                }
            }

            result
        } else {
            Err(
                crate::core::persistence::state_holder::StateError::DeserializationError {
                    message: "No state holder available".to_string(),
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
                crate::core::persistence::state_holder::StateError::CheckpointNotFound {
                    checkpoint_id: since,
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
                crate::core::persistence::state_holder::StateError::DeserializationError {
                    message: "No state holder available".to_string(),
                },
            )
        }
    }

    fn estimate_size(&self) -> crate::core::persistence::state_holder::StateSize {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.estimate_size()
        } else {
            crate::core::persistence::state_holder::StateSize {
                bytes: 8, // i64
                entries: 1,
                estimated_growth_rate: 0.0,
            }
        }
    }

    fn access_pattern(&self) -> crate::core::persistence::state_holder::AccessPattern {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.access_pattern()
        } else {
            crate::core::persistence::state_holder::AccessPattern::Random
        }
    }

    fn component_metadata(&self) -> crate::core::persistence::state_holder::StateMetadata {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.component_metadata()
        } else {
            crate::core::persistence::state_holder::StateMetadata::new(
                "unknown_count_aggregator".to_string(),
                "CountAttributeAggregatorExecutor".to_string(),
            )
        }
    }
}

#[derive(Debug, Clone, Default)]
struct DistinctCountState {
    map: HashMap<String, i64>,
}

#[derive(Debug)]
pub struct DistinctCountAttributeAggregatorExecutor {
    arg_exec: Option<Box<dyn ExpressionExecutor>>,
    state: Mutex<DistinctCountState>,
    app_ctx: Option<Arc<EventFluxAppContext>>,
    state_holder: Option<DistinctCountAggregatorStateHolder>,
}

impl Default for DistinctCountAttributeAggregatorExecutor {
    fn default() -> Self {
        Self {
            arg_exec: None,
            state: Mutex::new(DistinctCountState::default()),
            app_ctx: None,
            state_holder: None,
        }
    }
}

impl AttributeAggregatorExecutor for DistinctCountAttributeAggregatorExecutor {
    fn init(
        &mut self,
        mut e: Vec<Box<dyn ExpressionExecutor>>,
        _m: ProcessingMode,
        _ex: bool,
        ctx: &EventFluxQueryContext,
    ) -> Result<(), String> {
        if e.len() != 1 {
            return Err("distinctCount() requires one arg".to_string());
        }
        self.arg_exec = Some(e.remove(0));
        self.app_ctx = Some(Arc::clone(&ctx.eventflux_app_context));

        // Initialize StateHolder
        let map_ref = Arc::new(Mutex::new(HashMap::new()));
        let aggregator_id = ctx.next_aggregator_id();
        let component_id = format!("distinctcount_aggregator_{}", aggregator_id);
        self.state_holder = Some(DistinctCountAggregatorStateHolder::new(
            map_ref.clone(),
            component_id,
        ));

        Ok(())
    }
    fn process_add(&self, data: Option<AttributeValue>) -> Option<AttributeValue> {
        if let Some(v) = data {
            let key = format!("{v:?}");
            let mut st = self.state.lock().unwrap();
            let old_count = st.map.get(&key).copied();
            let c = st.map.entry(key.clone()).or_insert(0);
            *c += 1;

            // Record state change
            if let Some(ref state_holder) = self.state_holder {
                state_holder.record_value_added(&key, old_count, *c);
            }
        }
        Some(AttributeValue::Long(
            self.state.lock().unwrap().map.len() as i64
        ))
    }
    fn process_remove(&self, data: Option<AttributeValue>) -> Option<AttributeValue> {
        if let Some(v) = data {
            let key = format!("{v:?}");
            let mut st = self.state.lock().unwrap();
            if let Some(c) = st.map.get_mut(&key) {
                let old_count = *c;
                *c -= 1;
                let new_count = if *c <= 0 {
                    st.map.remove(&key);
                    None
                } else {
                    Some(*c)
                };

                // Record state change
                if let Some(ref state_holder) = self.state_holder {
                    state_holder.record_value_removed(&key, old_count, new_count);
                }
            }
        }
        Some(AttributeValue::Long(
            self.state.lock().unwrap().map.len() as i64
        ))
    }
    fn reset(&self) -> Option<AttributeValue> {
        let old_map = {
            let mut st = self.state.lock().unwrap();
            let old_map = st.map.clone();
            st.map.clear();
            old_map
        };

        // Record state change
        if let Some(ref state_holder) = self.state_holder {
            state_holder.record_reset(&old_map);
        }

        Some(AttributeValue::Long(0))
    }
    fn clone_box(&self) -> Box<dyn AttributeAggregatorExecutor> {
        let ctx = self.app_ctx.as_ref().unwrap();
        Box::new(DistinctCountAttributeAggregatorExecutor {
            arg_exec: self.arg_exec.as_ref().map(|e| e.clone_executor(ctx)),
            state: Mutex::new(self.state.lock().unwrap().clone()),
            app_ctx: Some(Arc::clone(ctx)),
            state_holder: self.state_holder.clone(),
        })
    }
}

impl ExpressionExecutor for DistinctCountAttributeAggregatorExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let event = event?;
        let data = self.arg_exec.as_ref().and_then(|e| e.execute(Some(event)));
        match event.get_event_type() {
            ComplexEventType::Current => self.process_add(data),
            ComplexEventType::Expired => self.process_remove(data),
            ComplexEventType::Reset => self.reset(),
            _ => None,
        }
    }
    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::LONG
    }
    fn clone_executor(&self, _ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(AttributeAggregatorExpressionExecutor::new(self.clone_box()))
    }
    fn is_attribute_aggregator(&self) -> bool {
        true
    }
}

// StateHolder implementation for DistinctCountAttributeAggregatorExecutor
impl crate::core::persistence::state_holder::StateHolder
    for DistinctCountAttributeAggregatorExecutor
{
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
                crate::core::persistence::state_holder::StateError::SerializationError {
                    message: "No state holder available".to_string(),
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
                crate::core::persistence::state_holder::StateError::DeserializationError {
                    message: "No state holder available".to_string(),
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
                crate::core::persistence::state_holder::StateError::CheckpointNotFound {
                    checkpoint_id: since,
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
                crate::core::persistence::state_holder::StateError::DeserializationError {
                    message: "No state holder available".to_string(),
                },
            )
        }
    }

    fn estimate_size(&self) -> crate::core::persistence::state_holder::StateSize {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.estimate_size()
        } else {
            crate::core::persistence::state_holder::StateSize {
                bytes: std::mem::size_of::<HashMap<String, i64>>(),
                entries: 0,
                estimated_growth_rate: 0.1,
            }
        }
    }

    fn access_pattern(&self) -> crate::core::persistence::state_holder::AccessPattern {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.access_pattern()
        } else {
            crate::core::persistence::state_holder::AccessPattern::Random
        }
    }

    fn component_metadata(&self) -> crate::core::persistence::state_holder::StateMetadata {
        if let Some(ref state_holder) = self.state_holder {
            state_holder.component_metadata()
        } else {
            crate::core::persistence::state_holder::StateMetadata::new(
                "unknown_distinctcount_aggregator".to_string(),
                "DistinctCountAttributeAggregatorExecutor".to_string(),
            )
        }
    }
}

#[derive(Debug, Clone, Default)]
struct MinMaxState {
    value: Option<f64>,
}

macro_rules! minmax_exec {
    ($name:ident, $cmp:expr) => {
        #[derive(Debug)]
        pub struct $name {
            arg_exec: Option<Box<dyn ExpressionExecutor>>,
            return_type: ApiAttributeType,
            state: Mutex<MinMaxState>,
            app_ctx: Option<Arc<EventFluxAppContext>>,
        }
        impl Default for $name {
            fn default() -> Self {
                Self {
                    arg_exec: None,
                    return_type: ApiAttributeType::DOUBLE,
                    state: Mutex::new(MinMaxState::default()),
                    app_ctx: None,
                }
            }
        }
        impl AttributeAggregatorExecutor for $name {
            fn init(
                &mut self,
                mut e: Vec<Box<dyn ExpressionExecutor>>,
                _m: ProcessingMode,
                _ex: bool,
                ctx: &EventFluxQueryContext,
            ) -> Result<(), String> {
                if e.len() != 1 {
                    return Err("aggregator requires one arg".into());
                }
                let exec = e.remove(0);
                self.return_type = exec.get_return_type();
                self.arg_exec = Some(exec);
                self.app_ctx = Some(Arc::clone(&ctx.eventflux_app_context));
                Ok(())
            }
            fn process_add(&self, data: Option<AttributeValue>) -> Option<AttributeValue> {
                if let Some(v) = data.and_then(|v| value_as_f64(&v)) {
                    let mut st = self.state.lock().unwrap();
                    match st.value {
                        Some(current) => {
                            if $cmp(v, current) {
                                st.value = Some(v);
                            }
                        }
                        None => st.value = Some(v),
                    }
                }
                st_to_val(self.return_type, &self.state.lock().unwrap())
            }
            fn process_remove(&self, _data: Option<AttributeValue>) -> Option<AttributeValue> {
                st_to_val(self.return_type, &self.state.lock().unwrap())
            }
            fn reset(&self) -> Option<AttributeValue> {
                self.state.lock().unwrap().value = None;
                None
            }
            fn clone_box(&self) -> Box<dyn AttributeAggregatorExecutor> {
                let ctx = self.app_ctx.as_ref().unwrap();
                Box::new($name {
                    arg_exec: self.arg_exec.as_ref().map(|e| e.clone_executor(ctx)),
                    return_type: self.return_type,
                    state: Mutex::new(self.state.lock().unwrap().clone()),
                    app_ctx: Some(Arc::clone(ctx)),
                })
            }
        }
        impl ExpressionExecutor for $name {
            fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
                let event = event?;
                let data = self.arg_exec.as_ref().and_then(|e| e.execute(Some(event)));
                match event.get_event_type() {
                    ComplexEventType::Current => self.process_add(data),
                    ComplexEventType::Expired => self.process_remove(data),
                    ComplexEventType::Reset => self.reset(),
                    _ => None,
                }
            }
            fn get_return_type(&self) -> ApiAttributeType {
                self.return_type
            }
            fn clone_executor(
                &self,
                _ctx: &Arc<EventFluxAppContext>,
            ) -> Box<dyn ExpressionExecutor> {
                Box::new(AttributeAggregatorExpressionExecutor::new(self.clone_box()))
            }
            fn is_attribute_aggregator(&self) -> bool {
                true
            }
        }
    };
}

fn st_to_val(rt: ApiAttributeType, st: &MinMaxState) -> Option<AttributeValue> {
    st.value.map(|v| match rt {
        ApiAttributeType::INT => AttributeValue::Int(v as i32),
        ApiAttributeType::LONG => AttributeValue::Long(v as i64),
        ApiAttributeType::FLOAT => AttributeValue::Float(v as f32),
        _ => AttributeValue::Double(v),
    })
}

minmax_exec!(MinAttributeAggregatorExecutor, |v, current| v < current);
minmax_exec!(MaxAttributeAggregatorExecutor, |v, current| v > current);
minmax_exec!(MinForeverAttributeAggregatorExecutor, |v, current| v
    < current);
minmax_exec!(MaxForeverAttributeAggregatorExecutor, |v, current| v
    > current);

use crate::core::extension::AttributeAggregatorFactory;

pub mod sum_aggregator_state_holder;
pub use sum_aggregator_state_holder::SumAggregatorStateHolder;

pub mod count_aggregator_state_holder;
pub use count_aggregator_state_holder::CountAggregatorStateHolder;

pub mod avg_aggregator_state_holder;
pub use avg_aggregator_state_holder::AvgAggregatorStateHolder;

pub mod min_aggregator_state_holder;
pub use min_aggregator_state_holder::MinAggregatorStateHolder;

pub mod max_aggregator_state_holder;
pub use max_aggregator_state_holder::MaxAggregatorStateHolder;

pub mod distinctcount_aggregator_state_holder;
pub use distinctcount_aggregator_state_holder::DistinctCountAggregatorStateHolder;

#[derive(Debug, Clone)]
pub struct SumAttributeAggregatorFactory;

impl AttributeAggregatorFactory for SumAttributeAggregatorFactory {
    fn name(&self) -> &'static str {
        "sum"
    }
    fn create(&self) -> Box<dyn AttributeAggregatorExecutor> {
        Box::new(SumAttributeAggregatorExecutor::default())
    }
    fn clone_box(&self) -> Box<dyn AttributeAggregatorFactory> {
        Box::new(Self)
    }
}

#[derive(Debug, Clone)]
pub struct AvgAttributeAggregatorFactory;

impl AttributeAggregatorFactory for AvgAttributeAggregatorFactory {
    fn name(&self) -> &'static str {
        "avg"
    }
    fn create(&self) -> Box<dyn AttributeAggregatorExecutor> {
        Box::new(AvgAttributeAggregatorExecutor::default())
    }
    fn clone_box(&self) -> Box<dyn AttributeAggregatorFactory> {
        Box::new(Self)
    }
}

#[derive(Debug, Clone)]
pub struct CountAttributeAggregatorFactory;

impl AttributeAggregatorFactory for CountAttributeAggregatorFactory {
    fn name(&self) -> &'static str {
        "count"
    }
    fn create(&self) -> Box<dyn AttributeAggregatorExecutor> {
        Box::new(CountAttributeAggregatorExecutor::default())
    }
    fn clone_box(&self) -> Box<dyn AttributeAggregatorFactory> {
        Box::new(Self)
    }
}

#[derive(Debug, Clone)]
pub struct DistinctCountAttributeAggregatorFactory;

impl AttributeAggregatorFactory for DistinctCountAttributeAggregatorFactory {
    fn name(&self) -> &'static str {
        "distinctCount"
    }
    fn create(&self) -> Box<dyn AttributeAggregatorExecutor> {
        Box::new(DistinctCountAttributeAggregatorExecutor::default())
    }
    fn clone_box(&self) -> Box<dyn AttributeAggregatorFactory> {
        Box::new(Self)
    }
}

#[derive(Debug, Clone)]
pub struct MinAttributeAggregatorFactory;

impl AttributeAggregatorFactory for MinAttributeAggregatorFactory {
    fn name(&self) -> &'static str {
        "min"
    }
    fn create(&self) -> Box<dyn AttributeAggregatorExecutor> {
        Box::new(MinAttributeAggregatorExecutor::default())
    }
    fn clone_box(&self) -> Box<dyn AttributeAggregatorFactory> {
        Box::new(Self)
    }
}

#[derive(Debug, Clone)]
pub struct MaxAttributeAggregatorFactory;

impl AttributeAggregatorFactory for MaxAttributeAggregatorFactory {
    fn name(&self) -> &'static str {
        "max"
    }
    fn create(&self) -> Box<dyn AttributeAggregatorExecutor> {
        Box::new(MaxAttributeAggregatorExecutor::default())
    }
    fn clone_box(&self) -> Box<dyn AttributeAggregatorFactory> {
        Box::new(Self)
    }
}

#[derive(Debug, Clone)]
pub struct MinForeverAttributeAggregatorFactory;

impl AttributeAggregatorFactory for MinForeverAttributeAggregatorFactory {
    fn name(&self) -> &'static str {
        "minForever"
    }
    fn create(&self) -> Box<dyn AttributeAggregatorExecutor> {
        Box::new(MinForeverAttributeAggregatorExecutor::default())
    }
    fn clone_box(&self) -> Box<dyn AttributeAggregatorFactory> {
        Box::new(Self)
    }
}

#[derive(Debug, Clone)]
pub struct MaxForeverAttributeAggregatorFactory;

impl AttributeAggregatorFactory for MaxForeverAttributeAggregatorFactory {
    fn name(&self) -> &'static str {
        "maxForever"
    }
    fn create(&self) -> Box<dyn AttributeAggregatorExecutor> {
        Box::new(MaxForeverAttributeAggregatorExecutor::default())
    }
    fn clone_box(&self) -> Box<dyn AttributeAggregatorFactory> {
        Box::new(Self)
    }
}
