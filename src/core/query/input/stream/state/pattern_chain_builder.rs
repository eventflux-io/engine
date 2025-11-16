// SPDX-License-Identifier: MIT OR Apache-2.0

//! Pattern Chain Builder - Factory for creating multi-processor pattern chains
//!
//! Creates and wires CountPreStateProcessor chains for patterns like A{2} -> B{2} -> C{2}.
//! Each step in the chain is a separate processor with unique state_id.
//!
//! Reference: feat/pattern_processing/STATE_MACHINE_DESIGN.md

use super::count_post_state_processor::CountPostStateProcessor;
use super::count_pre_state_processor::CountPreStateProcessor;
use super::post_state_processor::PostStateProcessor;
use super::pre_state_processor::PreStateProcessor;
use super::stream_pre_state_processor::StateType;
use crate::core::config::eventflux_app_context::EventFluxAppContext;
use crate::core::config::eventflux_query_context::EventFluxQueryContext;
use std::sync::{Arc, Mutex};

/// Configuration for a single pattern step
#[derive(Debug, Clone)]
pub struct PatternStepConfig {
    /// Event alias (e1, e2, etc.)
    pub alias: String,
    /// Stream name to match
    pub stream_name: String,
    /// Minimum count required
    pub min_count: usize,
    /// Maximum count allowed
    pub max_count: usize,
}

impl PatternStepConfig {
    pub fn new(alias: String, stream_name: String, min_count: usize, max_count: usize) -> Self {
        Self {
            alias,
            stream_name,
            min_count,
            max_count,
        }
    }

    /// Validate this step's constraints
    pub fn validate(&self) -> Result<(), String> {
        if self.min_count > self.max_count {
            return Err(format!(
                "Step '{}': min_count ({}) cannot be greater than max_count ({})",
                self.alias, self.min_count, self.max_count
            ));
        }
        if self.min_count == 0 {
            return Err(format!(
                "Step '{}': min_count must be >= 1 (got 0)",
                self.alias
            ));
        }
        Ok(())
    }
}

/// Pattern chain builder for creating multi-processor chains
pub struct PatternChainBuilder {
    steps: Vec<PatternStepConfig>,
    state_type: StateType,
    within_duration_ms: Option<i64>,
}

impl PatternChainBuilder {
    pub fn new(state_type: StateType) -> Self {
        Self {
            steps: Vec::new(),
            state_type,
            within_duration_ms: None,
        }
    }

    pub fn add_step(&mut self, step: PatternStepConfig) {
        self.steps.push(step);
    }

    pub fn set_within(&mut self, duration_ms: i64) {
        self.within_duration_ms = Some(duration_ms);
    }

    /// Validate pattern chain constraints
    pub fn validate(&self) -> Result<(), String> {
        if self.steps.is_empty() {
            return Err("Pattern chain must have at least one step".to_string());
        }

        // Validate each step individually
        for step in &self.steps {
            step.validate()?;
        }

        // All steps: min >= 1 (no zero-count steps allowed, including first step)
        for step in &self.steps {
            if step.min_count == 0 {
                return Err(format!(
                    "Step '{}' must have min_count >= 1 (got 0)",
                    step.alias
                ));
            }
        }

        // Last step: min == max (exact count)
        let last_idx = self.steps.len() - 1;
        if self.steps[last_idx].min_count != self.steps[last_idx].max_count {
            return Err(format!(
                "Last step '{}' must have exact count (min=max), got min={} max={}",
                self.steps[last_idx].alias,
                self.steps[last_idx].min_count,
                self.steps[last_idx].max_count
            ));
        }

        // All steps: min <= max (already enforced by PatternStepConfig.validate)

        Ok(())
    }

    /// Build the processor chain
    pub fn build(
        self,
        app_context: Arc<EventFluxAppContext>,
        query_context: Arc<EventFluxQueryContext>,
    ) -> Result<ProcessorChain, String> {
        self.validate()?;

        let mut pre_processors_concrete: Vec<Arc<Mutex<CountPreStateProcessor>>> = Vec::new();
        let mut post_processors_concrete: Vec<Arc<Mutex<CountPostStateProcessor>>> = Vec::new();

        // Create PreStateProcessors
        for (i, step) in self.steps.iter().enumerate() {
            let pre = Arc::new(Mutex::new(CountPreStateProcessor::new(
                step.min_count,
                step.max_count,
                i,      // state_id
                i == 0, // is_start_state
                self.state_type,
                app_context.clone(),
                query_context.clone(),
            )));

            // Set WITHIN on first processor
            if i == 0 {
                if let Some(within_ms) = self.within_duration_ms {
                    pre.lock().unwrap().set_within_time(within_ms);
                }
            }

            pre_processors_concrete.push(pre);
        }

        // Create PostStateProcessors and wire chain
        for i in 0..self.steps.len() {
            let post = Arc::new(Mutex::new(CountPostStateProcessor::new(
                self.steps[i].min_count,
                self.steps[i].max_count,
                i, // state_id
            )));

            // Wire Pre -> Post
            pre_processors_concrete[i]
                .lock()
                .unwrap()
                .stream_processor
                .set_this_state_post_processor(post.clone() as Arc<Mutex<dyn PostStateProcessor>>);

            // Wire Post -> Next Pre (for pattern chain forwarding A -> B)
            if i + 1 < self.steps.len() {
                post.lock().unwrap().set_next_state_pre_processor(
                    pre_processors_concrete[i + 1].clone() as Arc<Mutex<dyn PreStateProcessor>>,
                );
            }

            post_processors_concrete.push(post);
        }

        // Convert to trait objects for ProcessorChain
        let pre_processors: Vec<Arc<Mutex<dyn PreStateProcessor>>> = pre_processors_concrete
            .iter()
            .map(|p| p.clone() as Arc<Mutex<dyn PreStateProcessor>>)
            .collect();
        let post_processors: Vec<Arc<Mutex<dyn PostStateProcessor>>> = post_processors_concrete
            .iter()
            .map(|p| p.clone() as Arc<Mutex<dyn PostStateProcessor>>)
            .collect();

        // Clone first processor before moving the vector
        let first_processor = pre_processors[0].clone();

        Ok(ProcessorChain {
            pre_processors,
            post_processors,
            first_processor,
            pre_processors_concrete,
        })
    }
}

/// Processor chain holding all wired processors
pub struct ProcessorChain {
    pub pre_processors: Vec<Arc<Mutex<dyn PreStateProcessor>>>,
    pub post_processors: Vec<Arc<Mutex<dyn PostStateProcessor>>>,
    pub first_processor: Arc<Mutex<dyn PreStateProcessor>>,
    // Keep concrete types for setup and test access
    pub pre_processors_concrete: Vec<Arc<Mutex<CountPreStateProcessor>>>,
}

impl ProcessorChain {
    /// Initialize all processors
    pub fn init(&mut self) {
        for pre in &self.pre_processors {
            pre.lock().unwrap().init();
        }
    }

    /// Set up stream and state event cloners for all processors
    ///
    /// Must be called before using the chain to process events
    pub fn setup_cloners(
        &mut self,
        stream_defs: Vec<Arc<crate::query_api::definition::stream_definition::StreamDefinition>>,
    ) {
        use crate::core::event::state::meta_state_event::MetaStateEvent;
        use crate::core::event::state::state_event_cloner::StateEventCloner;
        use crate::core::event::state::state_event_factory::StateEventFactory;
        use crate::core::event::stream::meta_stream_event::MetaStreamEvent;
        use crate::core::event::stream::stream_event_cloner::StreamEventCloner;
        use crate::core::event::stream::stream_event_factory::StreamEventFactory;

        let num_steps = self.pre_processors_concrete.len();

        for (i, pre) in self.pre_processors_concrete.iter().enumerate() {
            // Set up stream event cloner
            let stream_def = if i < stream_defs.len() {
                stream_defs[i].clone()
            } else {
                stream_defs[0].clone() // Fallback to first def
            };

            let meta_stream = MetaStreamEvent::new_for_single_input(stream_def);
            let stream_factory = StreamEventFactory::new(i, 0, 0);
            let stream_cloner = StreamEventCloner::new(&meta_stream, stream_factory);

            // Set up state event cloner
            let meta_state = MetaStateEvent::new(num_steps);
            let state_factory = StateEventFactory::new(num_steps, 0);
            let state_cloner = StateEventCloner::new(&meta_state, state_factory);

            // Set cloners on the processor
            let mut pre_locked = pre.lock().unwrap();
            pre_locked
                .stream_processor
                .set_stream_event_cloner(stream_cloner);
            pre_locked
                .stream_processor
                .set_state_event_cloner(state_cloner);
        }
    }

    /// Expire events in all processors
    pub fn expire_events(&mut self, timestamp: i64) {
        for pre in &self.pre_processors {
            pre.lock().unwrap().expire_events(timestamp);
        }
    }

    /// Update state in all processors (moves new_list to pending_list)
    ///
    /// Must be called after events are processed to ensure forwarded states
    /// are moved from new_list to pending_list in all processors
    pub fn update_state(&mut self) {
        for pre in &self.pre_processors {
            pre.lock().unwrap().update_state();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_contexts() -> (Arc<EventFluxAppContext>, Arc<EventFluxQueryContext>) {
        let app_ctx = Arc::new(EventFluxAppContext::default_for_testing());
        let query_ctx = Arc::new(EventFluxQueryContext::new(
            app_ctx.clone(),
            "test_query".to_string(),
            None,
        ));
        (app_ctx, query_ctx)
    }

    #[test]
    fn test_pattern_step_config_creation() {
        let step = PatternStepConfig::new("e1".to_string(), "TempStream".to_string(), 1, 3);
        assert_eq!(step.alias, "e1");
        assert_eq!(step.stream_name, "TempStream");
        assert_eq!(step.min_count, 1);
        assert_eq!(step.max_count, 3);
    }

    #[test]
    fn test_pattern_step_config_validation_success() {
        let step = PatternStepConfig::new("e1".to_string(), "S".to_string(), 2, 5);
        assert!(step.validate().is_ok());
    }

    #[test]
    fn test_pattern_step_config_validation_fail_min_greater_than_max() {
        let step = PatternStepConfig::new("e1".to_string(), "S".to_string(), 5, 2);
        let result = step.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("min_count"));
    }

    #[test]
    fn test_pattern_step_config_validation_fail_min_zero() {
        let step = PatternStepConfig::new("e1".to_string(), "S".to_string(), 0, 2);
        let result = step.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("min_count must be >= 1"));
    }

    #[test]
    fn test_pattern_chain_builder_creation() {
        let builder = PatternChainBuilder::new(StateType::Sequence);
        assert_eq!(builder.steps.len(), 0);
    }

    #[test]
    fn test_pattern_chain_builder_add_step() {
        let mut builder = PatternChainBuilder::new(StateType::Sequence);
        builder.add_step(PatternStepConfig::new(
            "e1".to_string(),
            "A".to_string(),
            2,
            2,
        ));
        assert_eq!(builder.steps.len(), 1);
    }

    #[test]
    fn test_pattern_chain_validation_empty() {
        let builder = PatternChainBuilder::new(StateType::Sequence);
        let result = builder.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at least one step"));
    }

    #[test]
    fn test_pattern_chain_validation_first_step_min_zero() {
        let mut builder = PatternChainBuilder::new(StateType::Sequence);
        builder.add_step(PatternStepConfig::new(
            "e1".to_string(),
            "A".to_string(),
            0,
            2,
        ));
        builder.add_step(PatternStepConfig::new(
            "e2".to_string(),
            "B".to_string(),
            2,
            2,
        ));
        let result = builder.validate();
        assert!(result.is_err());
        // Error is caught at PatternStepConfig.validate() level before chain-level validation
        assert!(result.unwrap_err().contains("min_count must be >= 1"));
    }

    #[test]
    fn test_pattern_chain_validation_last_step_not_exact() {
        let mut builder = PatternChainBuilder::new(StateType::Sequence);
        builder.add_step(PatternStepConfig::new(
            "e1".to_string(),
            "A".to_string(),
            2,
            2,
        ));
        builder.add_step(PatternStepConfig::new(
            "e2".to_string(),
            "B".to_string(),
            1,
            5,
        ));
        let result = builder.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Last step"));
    }

    #[test]
    fn test_pattern_chain_validation_success() {
        let mut builder = PatternChainBuilder::new(StateType::Sequence);
        builder.add_step(PatternStepConfig::new(
            "e1".to_string(),
            "A".to_string(),
            2,
            2,
        ));
        builder.add_step(PatternStepConfig::new(
            "e2".to_string(),
            "B".to_string(),
            0,
            2,
        ));
        builder.add_step(PatternStepConfig::new(
            "e3".to_string(),
            "C".to_string(),
            2,
            2,
        ));
        // This should fail because middle step has min=0
        let result = builder.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_pattern_chain_validation_success_all_exact() {
        let mut builder = PatternChainBuilder::new(StateType::Sequence);
        builder.add_step(PatternStepConfig::new(
            "e1".to_string(),
            "A".to_string(),
            2,
            2,
        ));
        builder.add_step(PatternStepConfig::new(
            "e2".to_string(),
            "B".to_string(),
            2,
            2,
        ));
        builder.add_step(PatternStepConfig::new(
            "e3".to_string(),
            "C".to_string(),
            2,
            2,
        ));
        let result = builder.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_processor_chain_build_success() {
        let (app_ctx, query_ctx) = create_test_contexts();

        let mut builder = PatternChainBuilder::new(StateType::Sequence);
        builder.add_step(PatternStepConfig::new(
            "e1".to_string(),
            "A".to_string(),
            2,
            2,
        ));
        builder.add_step(PatternStepConfig::new(
            "e2".to_string(),
            "B".to_string(),
            2,
            2,
        ));

        let result = builder.build(app_ctx, query_ctx);
        assert!(result.is_ok());

        let chain = result.unwrap();
        assert_eq!(chain.pre_processors.len(), 2);
        assert_eq!(chain.post_processors.len(), 2);
    }

    #[test]
    fn test_processor_chain_build_fail_invalid() {
        let (app_ctx, query_ctx) = create_test_contexts();

        let mut builder = PatternChainBuilder::new(StateType::Sequence);
        builder.add_step(PatternStepConfig::new(
            "e1".to_string(),
            "A".to_string(),
            0,
            2,
        ));
        builder.add_step(PatternStepConfig::new(
            "e2".to_string(),
            "B".to_string(),
            2,
            2,
        ));

        let result = builder.build(app_ctx, query_ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_processor_chain_with_within() {
        let (app_ctx, query_ctx) = create_test_contexts();

        let mut builder = PatternChainBuilder::new(StateType::Sequence);
        builder.set_within(5000); // 5 seconds
        builder.add_step(PatternStepConfig::new(
            "e1".to_string(),
            "A".to_string(),
            2,
            2,
        ));
        builder.add_step(PatternStepConfig::new(
            "e2".to_string(),
            "B".to_string(),
            2,
            2,
        ));

        let result = builder.build(app_ctx, query_ctx);
        assert!(result.is_ok());
    }
}
