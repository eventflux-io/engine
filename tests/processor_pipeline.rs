// TODO: test_processor_pipeline converted to SQL but needs WHERE filter support
// Tests using programmatic API (not parser) remain enabled and passing.
// See feat/grammar/GRAMMAR_STATUS.md for M1 feature list.

#[path = "common/mod.rs"]
mod common;
use common::AppRunner;
use eventflux_rust::core::event::value::AttributeValue;

#[tokio::test]
#[ignore = "WHERE filter syntax not yet supported in SQL parser"]
async fn test_processor_pipeline() {
    // TODO: Converted to SQL syntax, but WHERE clause filtering not yet implemented
    let app = "\
        CREATE STREAM InputStream (a INT);\n\
        CREATE STREAM OutStream (a INT);\n\
        INSERT INTO OutStream\n\
        SELECT a FROM InputStream WHERE a > 10;\n";
    let runner = AppRunner::new(app, "OutStream").await;
    runner.send("InputStream", vec![AttributeValue::Int(5)]);
    runner.send("InputStream", vec![AttributeValue::Int(20)]);
    let out = runner.shutdown();
    assert_eq!(out, vec![vec![AttributeValue::Int(20)]]);
}

#[tokio::test]
async fn test_order_by_limit_offset() {
    use eventflux_rust::query_api::definition::{attribute::Type as AttrType, StreamDefinition};
    use eventflux_rust::query_api::eventflux_app::EventFluxApp;
    use eventflux_rust::query_api::execution::query::output::output_stream::{
        InsertIntoStreamAction, OutputStream, OutputStreamAction,
    };
    use eventflux_rust::query_api::execution::query::selection::order_by_attribute::Order;
    use eventflux_rust::query_api::execution::query::selection::Selector;
    use eventflux_rust::query_api::execution::query::{input::InputStream, Query};
    use eventflux_rust::query_api::execution::ExecutionElement;
    use eventflux_rust::query_api::expression::{constant::Constant, variable::Variable};

    let mut app = EventFluxApp::new("App".to_string());
    app.add_stream_definition(
        StreamDefinition::new("InputStream".to_string()).attribute("a".to_string(), AttrType::INT),
    );
    app.add_stream_definition(
        StreamDefinition::new("OutStream".to_string()).attribute("a".to_string(), AttrType::INT),
    );

    let input = InputStream::stream("InputStream".to_string());
    let selector = Selector::new()
        .select_variable(Variable::new("a".to_string()))
        .order_by_with_order(Variable::new("a".to_string()), Order::Desc)
        .limit(Constant::int(2))
        .unwrap()
        .offset(Constant::int(1))
        .unwrap();
    let out_action = InsertIntoStreamAction {
        target_id: "OutStream".to_string(),
        is_inner_stream: false,
        is_fault_stream: false,
    };
    let out_stream = OutputStream::new(OutputStreamAction::InsertInto(out_action), None);
    let query = Query::query()
        .from(input)
        .select(selector)
        .out_stream(out_stream);
    app.add_execution_element(ExecutionElement::Query(query));

    let runner = AppRunner::new_from_api(app, "OutStream").await;
    runner.send_batch(
        "InputStream",
        vec![
            vec![AttributeValue::Int(1)],
            vec![AttributeValue::Int(5)],
            vec![AttributeValue::Int(3)],
            vec![AttributeValue::Int(2)],
        ],
    );
    let out = runner.shutdown();
    let expected = vec![vec![AttributeValue::Int(3)], vec![AttributeValue::Int(2)]];
    assert_eq!(out, expected);
}

#[tokio::test]
async fn test_group_by_order_by() {
    use eventflux_rust::query_api::definition::{attribute::Type as AttrType, StreamDefinition};
    use eventflux_rust::query_api::eventflux_app::EventFluxApp;
    use eventflux_rust::query_api::execution::query::output::output_stream::{
        InsertIntoStreamAction, OutputStream, OutputStreamAction,
    };
    use eventflux_rust::query_api::execution::query::selection::order_by_attribute::Order;
    use eventflux_rust::query_api::execution::query::selection::Selector;
    use eventflux_rust::query_api::execution::query::{input::InputStream, Query};
    use eventflux_rust::query_api::execution::ExecutionElement;
    use eventflux_rust::query_api::expression::variable::Variable;

    let mut app = EventFluxApp::new("App2".to_string());
    app.add_stream_definition(
        StreamDefinition::new("InputStream".to_string()).attribute("a".to_string(), AttrType::INT),
    );
    app.add_stream_definition(
        StreamDefinition::new("OutStream".to_string()).attribute("a".to_string(), AttrType::INT),
    );

    let input = InputStream::stream("InputStream".to_string());
    let selector = Selector::new()
        .select_variable(Variable::new("a".to_string()))
        .group_by(Variable::new("a".to_string()))
        .order_by_with_order(Variable::new("a".to_string()), Order::Asc);
    let out_action = InsertIntoStreamAction {
        target_id: "OutStream".to_string(),
        is_inner_stream: false,
        is_fault_stream: false,
    };
    let out_stream = OutputStream::new(OutputStreamAction::InsertInto(out_action), None);
    let query = Query::query()
        .from(input)
        .select(selector)
        .out_stream(out_stream);
    app.add_execution_element(ExecutionElement::Query(query));

    let runner = AppRunner::new_from_api(app, "OutStream").await;
    runner.send_batch(
        "InputStream",
        vec![
            vec![AttributeValue::Int(2)],
            vec![AttributeValue::Int(1)],
            vec![AttributeValue::Int(2)],
        ],
    );
    let out = runner.shutdown();
    let expected = vec![vec![AttributeValue::Int(1)], vec![AttributeValue::Int(2)]];
    assert_eq!(out, expected);
}
