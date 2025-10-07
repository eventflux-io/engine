use std::sync::{Arc, Mutex};

use crate::core::config::{
    eventflux_app_context::EventFluxAppContext, eventflux_query_context::EventFluxQueryContext,
};
use crate::core::event::complex_event::ComplexEvent;
use crate::core::event::stream::stream_event::StreamEvent;
use crate::core::event::value::AttributeValue;
use crate::core::executor::expression_executor::ExpressionExecutor;
use crate::core::query::processor::{CommonProcessorMeta, ProcessingMode, Processor};
use crate::core::table::{CompiledCondition, Table};
use crate::query_api::execution::query::input::stream::join_input_stream::Type as JoinType;

#[derive(Debug)]
pub struct TableJoinProcessor {
    meta: CommonProcessorMeta,
    pub join_type: JoinType,
    pub compiled_condition: Option<Arc<dyn CompiledCondition>>,
    pub condition_executor: Option<Box<dyn ExpressionExecutor>>,
    pub stream_attr_count: usize,
    pub table_attr_count: usize,
    pub table: Arc<dyn Table>,
    pub next_processor: Option<Arc<Mutex<dyn Processor>>>,
}

impl TableJoinProcessor {
    pub fn new(
        join_type: JoinType,
        compiled_condition: Option<Arc<dyn CompiledCondition>>,
        condition_executor: Option<Box<dyn ExpressionExecutor>>,
        stream_attr_count: usize,
        table_attr_count: usize,
        table: Arc<dyn Table>,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
    ) -> Self {
        Self {
            meta: CommonProcessorMeta::new(app_ctx, query_ctx),
            join_type,
            compiled_condition,
            condition_executor,
            stream_attr_count,
            table_attr_count,
            table,
            next_processor: None,
        }
    }

    fn build_joined_event(
        &self,
        stream: &StreamEvent,
        row: Option<&[AttributeValue]>,
    ) -> StreamEvent {
        let mut event = StreamEvent::new(
            stream.timestamp,
            self.stream_attr_count + self.table_attr_count,
            0,
            0,
        );
        for i in 0..self.stream_attr_count {
            event.before_window_data[i] = stream
                .before_window_data
                .get(i)
                .cloned()
                .unwrap_or(AttributeValue::Null);
        }
        for j in 0..self.table_attr_count {
            let val = row
                .and_then(|r| r.get(j).cloned())
                .unwrap_or(AttributeValue::Null);
            event.before_window_data[self.stream_attr_count + j] = val;
        }
        event
    }

    fn forward(&self, se: StreamEvent) {
        if let Some(ref next) = self.next_processor {
            next.lock().unwrap().process(Some(Box::new(se)));
        }
    }
}

impl Processor for TableJoinProcessor {
    fn process(&self, mut chunk: Option<Box<dyn ComplexEvent>>) {
        while let Some(mut ce) = chunk {
            chunk = ce.set_next(None);
            if let Some(se) = ce.as_any().downcast_ref::<StreamEvent>() {
                let rows = self.table.find_rows_for_join(
                    se,
                    self.compiled_condition.as_deref(),
                    self.condition_executor.as_deref(),
                );
                let mut matched = false;
                for row in &rows {
                    matched = true;
                    let joined = self.build_joined_event(se, Some(row));
                    self.forward(joined);
                }
                if !matched && matches!(self.join_type, JoinType::LeftOuterJoin) {
                    let joined = self.build_joined_event(se, None);
                    self.forward(joined);
                }
            }
        }
    }

    fn next_processor(&self) -> Option<Arc<Mutex<dyn Processor>>> {
        self.next_processor.clone()
    }

    fn set_next_processor(&mut self, next: Option<Arc<Mutex<dyn Processor>>>) {
        self.next_processor = next;
    }

    fn clone_processor(&self, ctx: &Arc<EventFluxQueryContext>) -> Box<dyn Processor> {
        Box::new(TableJoinProcessor::new(
            self.join_type,
            self.compiled_condition.as_ref().map(Arc::clone),
            self.condition_executor
                .as_ref()
                .map(|c| c.clone_executor(&self.meta.eventflux_app_context)),
            self.stream_attr_count,
            self.table_attr_count,
            Arc::clone(&self.table),
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
        true
    }
}
