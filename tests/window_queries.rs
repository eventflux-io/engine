// SPDX-License-Identifier: MIT OR Apache-2.0

use eventflux_rust::core::config::{
    eventflux_app_context::EventFluxAppContext, eventflux_context::EventFluxContext,
};
use eventflux_rust::core::event::event::Event;
use eventflux_rust::core::event::value::AttributeValue;
use eventflux_rust::core::eventflux_manager::EventFluxManager;
use eventflux_rust::core::stream::output::stream_callback::StreamCallback;
use eventflux_rust::core::stream::stream_junction::StreamJunction;
use eventflux_rust::core::util::parser::QueryParser;
use eventflux_rust::query_api::definition::attribute::Type as AttrType;
use eventflux_rust::query_api::definition::StreamDefinition;
use eventflux_rust::query_api::execution::query::input::stream::input_stream::InputStream;
use eventflux_rust::query_api::execution::query::input::stream::single_input_stream::SingleInputStream;
use eventflux_rust::query_api::execution::query::output::output_stream::{
    InsertIntoStreamAction, OutputStream, OutputStreamAction,
};
use eventflux_rust::query_api::execution::query::selection::Selector;
use eventflux_rust::query_api::execution::query::Query;
use eventflux_rust::query_api::expression::Expression;
use eventflux_rust::query_compiler::parse;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn setup_context() -> (
    Arc<EventFluxAppContext>,
    HashMap<String, Arc<Mutex<StreamJunction>>>,
) {
    let eventflux_context = Arc::new(EventFluxContext::new());
    let app = Arc::new(eventflux_rust::query_api::eventflux_app::EventFluxApp::new(
        "TestApp".to_string(),
    ));
    let app_ctx = Arc::new(EventFluxAppContext::new(
        Arc::clone(&eventflux_context),
        "TestApp".to_string(),
        Arc::clone(&app),
        String::new(),
    ));

    let in_def = Arc::new(
        StreamDefinition::new("InputStream".to_string())
            .attribute("val".to_string(), AttrType::INT),
    );
    let out_def = Arc::new(
        StreamDefinition::new("OutStream".to_string()).attribute("val".to_string(), AttrType::INT),
    );

    let in_junction = Arc::new(Mutex::new(
        StreamJunction::new(
            "InputStream".to_string(),
            Arc::clone(&in_def),
            Arc::clone(&app_ctx),
            1024,
            false,
            None,
        )
        .unwrap(),
    ));
    let out_junction = Arc::new(Mutex::new(
        StreamJunction::new(
            "OutStream".to_string(),
            Arc::clone(&out_def),
            Arc::clone(&app_ctx),
            1024,
            false,
            None,
        )
        .unwrap(),
    ));

    let mut map = HashMap::new();
    map.insert("InputStream".to_string(), in_junction);
    map.insert("OutStream".to_string(), out_junction);

    (app_ctx, map)
}

#[derive(Debug)]
struct CollectEvents {
    events: Arc<Mutex<Vec<Event>>>,
}

impl StreamCallback for CollectEvents {
    fn receive_events(&self, events: &[Event]) {
        self.events.lock().unwrap().extend_from_slice(events);
    }
}

#[test]
fn test_length_window_query_parse() {
    let (app_ctx, junctions) = setup_context();
    let si =
        SingleInputStream::new_basic("InputStream".to_string(), false, false, None, Vec::new())
            .window(None, "length".to_string(), vec![Expression::value_int(5)]);
    let input = InputStream::Single(si);
    let selector = Selector::new();
    let insert_action = InsertIntoStreamAction {
        target_id: "OutStream".to_string(),
        is_inner_stream: false,
        is_fault_stream: false,
    };
    let out_stream = OutputStream::new(OutputStreamAction::InsertInto(insert_action), None);
    let query = Query::query()
        .from(input)
        .select(selector)
        .out_stream(out_stream);

    let res = QueryParser::parse_query_test(
        &query,
        &app_ctx,
        &junctions,
        &HashMap::new(),
        &HashMap::new(),
        None,
    );
    assert!(res.is_ok());
}

#[test]
fn test_time_window_query_parse() {
    let (app_ctx, junctions) = setup_context();
    let si =
        SingleInputStream::new_basic("InputStream".to_string(), false, false, None, Vec::new())
            .window(None, "time".to_string(), vec![Expression::time_sec(1)]);
    let input = InputStream::Single(si);
    let selector = Selector::new();
    let insert_action = InsertIntoStreamAction {
        target_id: "OutStream".to_string(),
        is_inner_stream: false,
        is_fault_stream: false,
    };
    let out_stream = OutputStream::new(OutputStreamAction::InsertInto(insert_action), None);
    let query = Query::query()
        .from(input)
        .select(selector)
        .out_stream(out_stream);

    let res = QueryParser::parse_query_test(
        &query,
        &app_ctx,
        &junctions,
        &HashMap::new(),
        &HashMap::new(),
        None,
    );
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_length_window_runtime() {
    let app = "\
        define stream In (v int);\n\
        define stream Out (v int);\n\
        from In#window:length(2) select v insert into Out;\n";
    let manager = EventFluxManager::new();
    let api = parse(app).expect("parse");
    let runtime = manager
        .create_eventflux_app_runtime_from_api(Arc::new(api), None)
        .await
        .expect("runtime");
    let collected = Arc::new(Mutex::new(Vec::new()));
    runtime
        .add_callback(
            "Out",
            Box::new(CollectEvents {
                events: Arc::clone(&collected),
            }),
        )
        .unwrap();
    runtime.start();
    let handler = runtime.get_input_handler("In").unwrap();
    handler
        .lock()
        .unwrap()
        .send_data(vec![AttributeValue::Int(1)])
        .unwrap();
    handler
        .lock()
        .unwrap()
        .send_data(vec![AttributeValue::Int(2)])
        .unwrap();
    handler
        .lock()
        .unwrap()
        .send_data(vec![AttributeValue::Int(3)])
        .unwrap();
    std::thread::sleep(Duration::from_millis(50));
    runtime.shutdown();
    let out = collected.lock().unwrap().clone();
    assert_eq!(out.len(), 4);
    assert_eq!(out[0].data, vec![AttributeValue::Int(1)]);
    assert_eq!(out[1].data, vec![AttributeValue::Int(2)]);
    assert!(out[2].is_expired);
    assert_eq!(out[2].data, vec![AttributeValue::Int(1)]);
    assert_eq!(out[3].data, vec![AttributeValue::Int(3)]);
}

#[tokio::test]
async fn test_time_window_runtime() {
    let app = "\
        define stream In (v int);\n\
        define stream Out (v int);\n\
        from In#window:time(100) select v insert into Out;\n";
    let manager = EventFluxManager::new();
    let api = parse(app).expect("parse");
    let runtime = manager
        .create_eventflux_app_runtime_from_api(Arc::new(api), None)
        .await
        .expect("runtime");
    let collected = Arc::new(Mutex::new(Vec::new()));
    runtime
        .add_callback(
            "Out",
            Box::new(CollectEvents {
                events: Arc::clone(&collected),
            }),
        )
        .unwrap();
    runtime.start();
    let handler = runtime.get_input_handler("In").unwrap();
    handler
        .lock()
        .unwrap()
        .send_data(vec![AttributeValue::Int(5)])
        .unwrap();
    std::thread::sleep(Duration::from_millis(150));
    runtime.shutdown();
    let out = collected.lock().unwrap().clone();
    assert!(out.len() >= 2);
    assert_eq!(out[0].data, vec![AttributeValue::Int(5)]);
    assert!(out.iter().any(|e| e.is_expired));
}

#[tokio::test]
async fn test_length_batch_window_runtime() {
    let app = "\
        define stream In (v int);\n\
        define stream Out (v int);\n\
        from In#window:lengthBatch(2) select v insert into Out;\n";
    let manager = EventFluxManager::new();
    let api = parse(app).expect("parse");
    let runtime = manager
        .create_eventflux_app_runtime_from_api(Arc::new(api), None)
        .await
        .expect("runtime");
    let collected = Arc::new(Mutex::new(Vec::new()));
    runtime
        .add_callback(
            "Out",
            Box::new(CollectEvents {
                events: Arc::clone(&collected),
            }),
        )
        .unwrap();
    runtime.start();
    let handler = runtime.get_input_handler("In").unwrap();
    handler
        .lock()
        .unwrap()
        .send_data(vec![AttributeValue::Int(1)])
        .unwrap();
    handler
        .lock()
        .unwrap()
        .send_data(vec![AttributeValue::Int(2)])
        .unwrap();
    handler
        .lock()
        .unwrap()
        .send_data(vec![AttributeValue::Int(3)])
        .unwrap();
    handler
        .lock()
        .unwrap()
        .send_data(vec![AttributeValue::Int(4)])
        .unwrap();
    std::thread::sleep(Duration::from_millis(50));
    runtime.shutdown();
    let out = collected.lock().unwrap().clone();
    assert!(out.len() >= 6);
    assert_eq!(out[0].data, vec![AttributeValue::Int(1)]);
    assert_eq!(out[1].data, vec![AttributeValue::Int(2)]);
    assert!(out[2].is_expired);
    assert!(out[3].is_expired);
    assert_eq!(out[4].data, vec![AttributeValue::Int(3)]);
    assert_eq!(out[5].data, vec![AttributeValue::Int(4)]);
}

#[tokio::test]
async fn test_time_batch_window_runtime() {
    let app = "\
        define stream In (v int);\n\
        define stream Out (v int);\n\
        from In#window:timeBatch(100) select v insert into Out;\n";
    let manager = EventFluxManager::new();
    let api = parse(app).expect("parse");
    let runtime = manager
        .create_eventflux_app_runtime_from_api(Arc::new(api), None)
        .await
        .expect("runtime");
    let collected = Arc::new(Mutex::new(Vec::new()));
    runtime
        .add_callback(
            "Out",
            Box::new(CollectEvents {
                events: Arc::clone(&collected),
            }),
        )
        .unwrap();
    runtime.start();
    let handler = runtime.get_input_handler("In").unwrap();
    handler
        .lock()
        .unwrap()
        .send_data(vec![AttributeValue::Int(1)])
        .unwrap();
    std::thread::sleep(Duration::from_millis(120));
    handler
        .lock()
        .unwrap()
        .send_data(vec![AttributeValue::Int(2)])
        .unwrap();
    std::thread::sleep(Duration::from_millis(120));
    runtime.shutdown();
    let out = collected.lock().unwrap().clone();
    assert!(out.len() >= 3);
    assert_eq!(out[0].data, vec![AttributeValue::Int(1)]);
    assert!(out.iter().any(|e| e.is_expired));
}
