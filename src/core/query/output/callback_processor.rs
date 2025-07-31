// siddhi_rust/src/core/query/output/callback_processor.rs
use crate::core::config::siddhi_app_context::SiddhiAppContext;
use crate::core::config::siddhi_query_context::SiddhiQueryContext;
use crate::core::event::complex_event::ComplexEvent;
use crate::core::event::event::Event; // For converting back
use crate::core::query::processor::{CommonProcessorMeta, ProcessingMode, Processor};
use crate::core::stream::output::stream_callback::StreamCallback; // The trait
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct CallbackProcessor {
    meta: CommonProcessorMeta,
    callback: Arc<Mutex<Box<dyn StreamCallback>>>, // Callback is now Boxed and behind Arc<Mutex<>>
}

impl CallbackProcessor {
    pub fn new(
        callback: Arc<Mutex<Box<dyn StreamCallback>>>,
        app_ctx: Arc<SiddhiAppContext>,
        query_ctx: Arc<SiddhiQueryContext>,
        // query_name: String, // query_name is in query_ctx
    ) -> Self {
        // query_name for CommonProcessorMeta can be extracted from query_ctx
        Self {
            meta: CommonProcessorMeta::new(app_ctx, query_ctx),
            callback,
        }
    }
}

// Helper to convert a Box<dyn ComplexEvent> to a core::Event
// This is lossy if ComplexEvent has more structure than Event.
// It primarily extracts output_data, timestamp, and type.
fn complex_event_to_simple_event(ce_box: Box<dyn ComplexEvent>) -> Event {
    let data = ce_box
        .get_output_data()
        .map_or_else(Vec::new, |d| d.to_vec());
    Event {
        id: 0, // Siddhi core Event does not have an ID like query_api::Event.
        // Or generate a new one if needed for callback context.
        // For now, 0 as placeholder.
        timestamp: ce_box.get_timestamp(),
        data,
        is_expired: ce_box.is_expired(),
    }
}

impl Processor for CallbackProcessor {
    fn process(&self, complex_event_chunk: Option<Box<dyn ComplexEvent>>) {
        let mut events_vec: Vec<Event> = Vec::new();
        let mut current_opt = complex_event_chunk; // Takes ownership of the head of the chunk

        while let Some(mut current_box) = current_opt {
            // Detach the current event from the chain to process it individually
            let next_event_in_chunk = current_box.set_next(None);
            events_vec.push(complex_event_to_simple_event(current_box));
            current_opt = next_event_in_chunk;
        }

        if !events_vec.is_empty() {
            // Lock the Mutex to call receive.
            // The receive method takes Vec<Event>, not &[Event].
            self.callback
                .lock()
                .expect("Callback Mutex poisoned")
                .receive_events(&events_vec);
        }
    }

    fn next_processor(&self) -> Option<Arc<Mutex<dyn Processor>>> {
        None
    } // Terminal processor
    fn set_next_processor(&mut self, _next: Option<Arc<Mutex<dyn Processor>>>) {
        /* no-op */
    }

    fn clone_processor(
        &self,
        siddhi_query_context: &Arc<SiddhiQueryContext>,
    ) -> Box<dyn Processor> {
        Box::new(Self::new(
            Arc::clone(&self.callback),
            Arc::clone(&self.meta.siddhi_app_context),
            Arc::clone(siddhi_query_context), // Use the new query context for the clone
                                              // self.meta.query_name.clone() // query_name is in siddhi_query_context
        ))
    }

    fn get_siddhi_app_context(&self) -> Arc<SiddhiAppContext> {
        Arc::clone(&self.meta.siddhi_app_context)
    }
    fn get_siddhi_query_context(&self) -> Arc<SiddhiQueryContext> {
        self.meta.get_siddhi_query_context()
    }
    fn get_processing_mode(&self) -> ProcessingMode {
        // Callbacks generally operate in default/pass-through mode relative to the query's output.
        ProcessingMode::DEFAULT
    }
    fn is_stateful(&self) -> bool {
        false // CallbackProcessor itself is typically stateless; the callback it holds might be.
    }
}
