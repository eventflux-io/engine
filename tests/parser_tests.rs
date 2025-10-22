// SPDX-License-Identifier: MIT OR Apache-2.0

// TODO: Some tests converted to SQL syntax but still disabled due to missing features
// Tests using programmatic API (not parser) remain enabled and passing.
// See feat/grammar/GRAMMAR_STATUS.md for M1 feature list.

#[path = "common/mod.rs"]
mod common;
use common::AppRunner;
use eventflux_rust::core::event::value::AttributeValue;

#[tokio::test]
#[ignore = "WHERE filter syntax not yet supported in SQL parser"]
async fn test_filter_projection() {
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
async fn test_length_window() {
    // Converted to SQL syntax - length window is M1 feature
    let app = "\
        CREATE STREAM In (v INT);\n\
        CREATE STREAM Out (v INT);\n\
        INSERT INTO Out\n\
        SELECT v FROM In WINDOW('length', 2);\n";
    let runner = AppRunner::new(app, "Out").await;
    runner.send("In", vec![AttributeValue::Int(1)]);
    runner.send("In", vec![AttributeValue::Int(2)]);
    runner.send("In", vec![AttributeValue::Int(3)]);
    let out = runner.shutdown();
    assert_eq!(
        out,
        vec![
            vec![AttributeValue::Int(1)],
            vec![AttributeValue::Int(2)],
            vec![AttributeValue::Int(1)],
            vec![AttributeValue::Int(3)],
        ]
    );
}

#[tokio::test]
async fn test_sum_aggregation() {
    // Converted to SQL syntax - sum aggregation is M1 feature
    let app = "\
        CREATE STREAM InStream (v INT);\n\
        CREATE STREAM OutStream (total BIGINT);\n\
        INSERT INTO OutStream\n\
        SELECT SUM(v) as total FROM InStream;\n";
    let runner = AppRunner::new(app, "OutStream").await;
    runner.send("InStream", vec![AttributeValue::Int(2)]);
    runner.send("InStream", vec![AttributeValue::Int(3)]);
    let out = runner.shutdown();
    assert_eq!(
        out,
        vec![vec![AttributeValue::Long(2)], vec![AttributeValue::Long(5)]]
    );
}

#[tokio::test]
async fn test_join_query() {
    // Converted to SQL JOIN syntax - JOINs are M1 feature and now working
    let app = "\
        CREATE STREAM Left (a INT);\n\
        CREATE STREAM Right (a INT);\n\
        CREATE STREAM Out (a INT, b INT);\n\
        INSERT INTO Out\n\
        SELECT Left.a as a, Right.a as b FROM Left JOIN Right ON Left.a = Right.a;\n";
    let runner = AppRunner::new(app, "Out").await;
    runner.send("Left", vec![AttributeValue::Int(5)]);
    runner.send("Right", vec![AttributeValue::Int(5)]);
    let out = runner.shutdown();
    assert_eq!(
        out,
        vec![vec![AttributeValue::Int(5), AttributeValue::Int(5)]]
    );
}

#[tokio::test]
#[ignore = "Namespaced functions (str:length) not yet supported - needs LENGTH() or similar"]
async fn test_builtin_function_in_query() {
    // TODO: Converted to SQL syntax, but str:length() function not in M1
    // Need to determine if we support LENGTH() or need to implement str namespace
    let app = "\
        CREATE STREAM In (v VARCHAR);\n\
        CREATE STREAM Out (len INT);\n\
        INSERT INTO Out\n\
        SELECT LENGTH(v) as len FROM In;\n";
    let runner = AppRunner::new(app, "Out").await;
    runner.send("In", vec![AttributeValue::String("abc".to_string())]);
    let out = runner.shutdown();
    assert_eq!(out, vec![vec![AttributeValue::Int(3)]]);
}

#[tokio::test]
async fn test_udf_in_query() {
    use eventflux_rust::core::config::eventflux_app_context::EventFluxAppContext;
    use eventflux_rust::core::eventflux_manager::EventFluxManager;
    use eventflux_rust::core::executor::expression_executor::ExpressionExecutor;
    use eventflux_rust::core::executor::function::scalar_function_executor::ScalarFunctionExecutor;
    use eventflux_rust::query_api::definition::attribute::Type as AttrType;

    use eventflux_rust::query_compiler::parse;
    use std::sync::Arc;

    #[derive(Debug, Default)]
    struct PlusOneFn {
        arg: Option<Box<dyn ExpressionExecutor>>,
    }

    impl Clone for PlusOneFn {
        fn clone(&self) -> Self {
            Self { arg: None }
        }
    }

    impl ExpressionExecutor for PlusOneFn {
        fn execute(
            &self,
            event: Option<&dyn eventflux_rust::core::event::complex_event::ComplexEvent>,
        ) -> Option<AttributeValue> {
            let v = self.arg.as_ref()?.execute(event)?;
            match v {
                AttributeValue::Int(i) => Some(AttributeValue::Int(i + 1)),
                _ => None,
            }
        }
        fn get_return_type(&self) -> AttrType {
            AttrType::INT
        }
        fn clone_executor(&self, _ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
            Box::new(self.clone())
        }
    }
    impl ScalarFunctionExecutor for PlusOneFn {
        fn init(
            &mut self,
            args: &Vec<Box<dyn ExpressionExecutor>>,
            ctx: &Arc<EventFluxAppContext>,
        ) -> Result<(), String> {
            if args.len() != 1 {
                return Err("plusOne expects one argument".to_string());
            }
            self.arg = Some(args[0].clone_executor(ctx));
            Ok(())
        }
        fn destroy(&mut self) {}
        fn get_name(&self) -> String {
            "plusOne".to_string()
        }
        fn clone_scalar_function(&self) -> Box<dyn ScalarFunctionExecutor> {
            Box::new(self.clone())
        }
    }

    let manager = EventFluxManager::new();
    manager.add_scalar_function_factory("plusOne".to_string(), Box::new(PlusOneFn::default()));

    let app_str = "\
        define stream In (v int);\n\
        define stream Out (v int);\n\
        from In select plusOne(v) as v insert into Out;\n";
    use eventflux_rust::core::event::event::Event;
    use eventflux_rust::core::stream::output::stream_callback::StreamCallback;
    use std::sync::Mutex;

    #[derive(Debug)]
    struct CollectCB {
        ev: Arc<Mutex<Vec<Vec<AttributeValue>>>>,
    }
    impl StreamCallback for CollectCB {
        fn receive_events(&self, events: &[Event]) {
            self.ev
                .lock()
                .unwrap()
                .extend(events.iter().map(|e| e.data.clone()));
        }
    }

    let api = parse(app_str).unwrap();
    let runtime = manager
        .create_eventflux_app_runtime_from_api(Arc::new(api), None)
        .await
        .unwrap();
    let collected = Arc::new(Mutex::new(Vec::new()));
    runtime
        .add_callback(
            "Out",
            Box::new(CollectCB {
                ev: Arc::clone(&collected),
            }),
        )
        .unwrap();
    runtime.start();
    let handler = runtime.get_input_handler("In").unwrap();
    handler
        .lock()
        .unwrap()
        .send_event_with_timestamp(0, vec![AttributeValue::Int(4)])
        .unwrap();
    runtime.shutdown();
    let out = collected.lock().unwrap().clone();
    assert_eq!(out, vec![vec![AttributeValue::Int(5)]]);
}

#[test]
fn test_parse_function_definition() {
    use eventflux_rust::query_api::definition::attribute::Type;
    use eventflux_rust::query_compiler::parse_function_definition;
    let def = parse_function_definition("define function foo [rust] return int 'body'").unwrap();
    assert_eq!(def.id, "foo");
    assert_eq!(def.language, "rust");
    assert_eq!(def.return_type, Type::INT);
}

#[test]
fn test_query_with_group_by() {
    use eventflux_rust::query_api::execution::query::selection::order_by_attribute::Order;
    use eventflux_rust::query_api::execution::ExecutionElement;
    use eventflux_rust::query_compiler::parse;

    let app = "\
        define stream In (a int, b int);\n\
        define stream Out (s long);\n\
        from In select sum(a) as s group by b having sum(a) > 0 order by b desc limit 5 offset 1 insert into Out;\n";
    let sa = parse(app).unwrap();
    match &sa.get_execution_elements()[0] {
        ExecutionElement::Query(q) => {
            assert_eq!(q.selector.group_by_list.len(), 1);
            assert!(matches!(
                q.selector.order_by_list[0].get_order(),
                Order::Desc
            ));
            assert!(q.selector.limit.is_some());
            assert!(q.selector.offset.is_some());
        }
        _ => panic!("expected query"),
    }
}
