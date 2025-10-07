// SPDX-License-Identifier: MIT OR Apache-2.0

use eventflux_rust::core::config::eventflux_query_context::EventFluxQueryContext;
use eventflux_rust::core::config::{
    eventflux_app_context::{EventFluxAppContext, MetricsLevelPlaceholder},
    eventflux_context::EventFluxContext,
};
use eventflux_rust::core::event::event::Event;
use eventflux_rust::core::event::value::AttributeValue;
use eventflux_rust::core::query::processor::Processor;
use eventflux_rust::core::stream::output::InMemoryErrorStore;
use eventflux_rust::core::stream::stream_junction::{OnErrorAction, StreamJunction};
use eventflux_rust::query_api::definition::{attribute::Type as AttrType, StreamDefinition};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct RecordingProcessor {
    events: Arc<Mutex<Vec<Vec<AttributeValue>>>>,
}

impl Processor for RecordingProcessor {
    fn process(
        &self,
        mut chunk: Option<Box<dyn eventflux_rust::core::event::complex_event::ComplexEvent>>,
    ) {
        while let Some(mut ce) = chunk {
            chunk = ce.set_next(None);
            if let Some(se) = ce
                .as_any()
                .downcast_ref::<eventflux_rust::core::event::stream::StreamEvent>()
            {
                self.events
                    .lock()
                    .unwrap()
                    .push(se.before_window_data.clone());
            }
        }
    }
    fn next_processor(&self) -> Option<Arc<Mutex<dyn Processor>>> {
        None
    }
    fn set_next_processor(&mut self, _n: Option<Arc<Mutex<dyn Processor>>>) {}
    fn clone_processor(
        &self,
        _c: &Arc<eventflux_rust::core::config::eventflux_query_context::EventFluxQueryContext>,
    ) -> Box<dyn Processor> {
        Box::new(RecordingProcessor {
            events: Arc::clone(&self.events),
        })
    }
    fn get_eventflux_app_context(&self) -> Arc<EventFluxAppContext> {
        Arc::new(EventFluxAppContext::new(
            Arc::new(EventFluxContext::new()),
            "T".to_string(),
            Arc::new(eventflux_rust::query_api::eventflux_app::EventFluxApp::new(
                "T".to_string(),
            )),
            String::new(),
        ))
    }

    fn get_eventflux_query_context(&self) -> Arc<EventFluxQueryContext> {
        Arc::new(EventFluxQueryContext::new(
            Arc::new(EventFluxAppContext::new(
                Arc::new(EventFluxContext::new()),
                "T".to_string(),
                Arc::new(eventflux_rust::query_api::eventflux_app::EventFluxApp::new(
                    "T".to_string(),
                )),
                String::new(),
            )),
            "q".to_string(),
            None,
        ))
    }
    fn get_processing_mode(&self) -> eventflux_rust::core::query::processor::ProcessingMode {
        eventflux_rust::core::query::processor::ProcessingMode::DEFAULT
    }
    fn is_stateful(&self) -> bool {
        false
    }
}

#[test]
fn test_fault_stream_routing() {
    let eventflux_context = Arc::new(EventFluxContext::new());
    let app = Arc::new(eventflux_rust::query_api::eventflux_app::EventFluxApp::new(
        "App".to_string(),
    ));
    let mut app_ctx = EventFluxAppContext::new(
        Arc::clone(&eventflux_context),
        "App".to_string(),
        Arc::clone(&app),
        String::new(),
    );
    app_ctx.set_root_metrics_level(MetricsLevelPlaceholder::BASIC);
    let def =
        Arc::new(StreamDefinition::new("In".to_string()).attribute("a".to_string(), AttrType::INT));
    let fault_def = Arc::new(
        StreamDefinition::new("Fault".to_string()).attribute("a".to_string(), AttrType::INT),
    );
    let fault_junction = Arc::new(Mutex::new(StreamJunction::new(
        "Fault".to_string(),
        Arc::clone(&fault_def),
        Arc::new(app_ctx.clone()),
        8,
        false,
        None,
    )));
    let mut main_junction = StreamJunction::new(
        "In".to_string(),
        Arc::clone(&def),
        Arc::new(app_ctx.clone()),
        8,
        true,
        Some(Arc::clone(&fault_junction)),
    );
    main_junction.set_on_error_action(OnErrorAction::STREAM);
    let rec = Arc::new(Mutex::new(Vec::new()));
    fault_junction
        .lock()
        .unwrap()
        .subscribe(Arc::new(Mutex::new(RecordingProcessor {
            events: Arc::clone(&rec),
        })));
    main_junction.stop_processing();
    main_junction.send_event(Event::new_with_data(0, vec![AttributeValue::Int(1)]));
    assert_eq!(rec.lock().unwrap().len(), 1);
}

#[test]
fn test_error_store_routing() {
    let mut eventflux_context = EventFluxContext::new();
    let error_store = Arc::new(InMemoryErrorStore::new());
    eventflux_context.set_error_store(error_store.clone());
    let eventflux_context = Arc::new(eventflux_context);
    let app = Arc::new(eventflux_rust::query_api::eventflux_app::EventFluxApp::new(
        "App".to_string(),
    ));
    let app_ctx = EventFluxAppContext::new(
        Arc::clone(&eventflux_context),
        "App".to_string(),
        Arc::clone(&app),
        String::new(),
    );
    let def =
        Arc::new(StreamDefinition::new("In".to_string()).attribute("a".to_string(), AttrType::INT));
    let mut junction = StreamJunction::new(
        "In".to_string(),
        Arc::clone(&def),
        Arc::new(app_ctx),
        8,
        true,
        None,
    );
    junction.set_on_error_action(OnErrorAction::STORE);
    junction.stop_processing();
    junction.send_event(Event::new_with_data(0, vec![AttributeValue::Int(2)]));
    assert_eq!(error_store.errors().len(), 1);
}
