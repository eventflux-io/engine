use siddhi_rust::core::config::siddhi_app_context::SiddhiAppContext;
use siddhi_rust::core::config::siddhi_context::SiddhiContext;
use siddhi_rust::core::config::siddhi_query_context::SiddhiQueryContext;
use siddhi_rust::core::event::stream::meta_stream_event::MetaStreamEvent;
use siddhi_rust::core::util::parser::QueryParser;
use siddhi_rust::core::util::parser::{parse_expression, ExpressionParserContext};
use siddhi_rust::query_api::definition::{attribute::Type as AttrType, StreamDefinition};
use siddhi_rust::query_api::execution::query::input::state::{State, StateElement};
use siddhi_rust::query_api::execution::query::input::stream::state_input_stream::StateInputStream;
use siddhi_rust::query_api::execution::query::input::stream::{
    InputStream, JoinType, SingleInputStream,
};
use siddhi_rust::query_api::execution::query::Query;
use siddhi_rust::query_api::expression::condition::compare::Operator as CompareOp;
use siddhi_rust::query_api::expression::{Expression, Variable};
use siddhi_rust::query_api::siddhi_app::SiddhiApp;
use std::collections::HashMap;
use std::sync::Arc;

fn make_app_ctx() -> Arc<SiddhiAppContext> {
    Arc::new(SiddhiAppContext::new(
        Arc::new(SiddhiContext::default()),
        "test".to_string(),
        Arc::new(SiddhiApp::new("app".to_string())),
        String::new(),
    ))
}

fn make_query_ctx(name: &str) -> Arc<SiddhiQueryContext> {
    Arc::new(SiddhiQueryContext::new(
        make_app_ctx(),
        name.to_string(),
        None,
    ))
}

#[test]
fn test_parse_expression_multi_stream_variable() {
    let stream_a =
        StreamDefinition::new("A".to_string()).attribute("val".to_string(), AttrType::INT);
    let stream_b =
        StreamDefinition::new("B".to_string()).attribute("val".to_string(), AttrType::DOUBLE);

    let meta_a = Arc::new(MetaStreamEvent::new_for_single_input(Arc::new(stream_a)));
    let meta_b = Arc::new(MetaStreamEvent::new_for_single_input(Arc::new(stream_b)));

    let mut map = HashMap::new();
    map.insert("A".to_string(), Arc::clone(&meta_a));
    map.insert("B".to_string(), Arc::clone(&meta_b));

    let ctx = ExpressionParserContext {
        siddhi_app_context: make_app_ctx(),
        siddhi_query_context: make_query_ctx("Q1"),
        stream_meta_map: map,
        table_meta_map: HashMap::new(),
        window_meta_map: HashMap::new(),
        aggregation_meta_map: HashMap::new(),
        state_meta_map: HashMap::new(),
        default_source: "A".to_string(),
        query_name: "Q1",
    };

    let var_a = Variable::new("val".to_string()).of_stream("A".to_string());
    let var_b = Variable::new("val".to_string()).of_stream("B".to_string());
    let expr = Expression::compare(
        Expression::Variable(var_a),
        CompareOp::LessThan,
        Expression::Variable(var_b),
    );

    let exec = parse_expression(&expr, &ctx).expect("parse failed");
    assert_eq!(exec.get_return_type(), AttrType::BOOL);
}

#[test]
fn test_compare_type_coercion_int_double() {
    let ctx = ExpressionParserContext {
        siddhi_app_context: make_app_ctx(),
        siddhi_query_context: make_query_ctx("Q2"),
        stream_meta_map: HashMap::new(),
        table_meta_map: HashMap::new(),
        window_meta_map: HashMap::new(),
        aggregation_meta_map: HashMap::new(),
        state_meta_map: HashMap::new(),
        default_source: "dummy".to_string(),
        query_name: "Q2",
    };

    let expr = Expression::compare(
        Expression::value_int(5),
        CompareOp::LessThan,
        Expression::value_double(5.5),
    );
    let exec = parse_expression(&expr, &ctx).unwrap();
    let result = exec.execute(None);
    assert_eq!(
        result,
        Some(siddhi_rust::core::event::value::AttributeValue::Bool(true))
    );
}

#[test]
fn test_variable_not_found_error() {
    let stream_a =
        StreamDefinition::new("A".to_string()).attribute("val".to_string(), AttrType::INT);
    let meta_a = Arc::new(MetaStreamEvent::new_for_single_input(Arc::new(stream_a)));
    let mut map = HashMap::new();
    map.insert("A".to_string(), Arc::clone(&meta_a));

    let ctx = ExpressionParserContext {
        siddhi_app_context: make_app_ctx(),
        siddhi_query_context: make_query_ctx("Q3"),
        stream_meta_map: map,
        table_meta_map: HashMap::new(),
        window_meta_map: HashMap::new(),
        aggregation_meta_map: HashMap::new(),
        state_meta_map: HashMap::new(),
        default_source: "A".to_string(),
        query_name: "Q3",
    };

    let mut var_b = Variable::new("missing".to_string()).of_stream("A".to_string());
    var_b.siddhi_element.query_context_start_index = Some((10, 5));
    let expr = Expression::Variable(var_b);
    let err = parse_expression(&expr, &ctx).unwrap_err();
    assert_eq!(err.line, Some(10));
    assert_eq!(err.column, Some(5));
    assert_eq!(err.query_name, "Q3");
}

#[test]
fn test_table_variable_resolution() {
    use siddhi_rust::query_api::definition::TableDefinition;
    let table = TableDefinition::new("T".to_string()).attribute("val".to_string(), AttrType::INT);
    let stream_equiv = Arc::new(
        StreamDefinition::new("T".to_string()).attribute("val".to_string(), AttrType::INT),
    );
    let meta = Arc::new(MetaStreamEvent::new_for_single_input(stream_equiv));
    let mut table_map = HashMap::new();
    table_map.insert("T".to_string(), Arc::clone(&meta));

    let ctx = ExpressionParserContext {
        siddhi_app_context: make_app_ctx(),
        siddhi_query_context: make_query_ctx("Q4"),
        stream_meta_map: HashMap::new(),
        table_meta_map: table_map,
        window_meta_map: HashMap::new(),
        aggregation_meta_map: HashMap::new(),
        state_meta_map: HashMap::new(),
        default_source: "T".to_string(),
        query_name: "Q4",
    };

    let var = Variable::new("val".to_string()).of_stream("T".to_string());
    let expr = Expression::Variable(var);
    let exec = parse_expression(&expr, &ctx).unwrap();
    assert_eq!(exec.get_return_type(), AttrType::INT);
}

#[test]
fn test_custom_udf_plus_one() {
    use siddhi_rust::core::event::value::AttributeValue;
    use siddhi_rust::core::executor::expression_executor::ExpressionExecutor;
    use siddhi_rust::core::executor::function::scalar_function_executor::ScalarFunctionExecutor;
    use siddhi_rust::core::siddhi_manager::SiddhiManager;
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
            event: Option<&dyn siddhi_rust::core::event::complex_event::ComplexEvent>,
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
        fn clone_executor(&self, _ctx: &Arc<SiddhiAppContext>) -> Box<dyn ExpressionExecutor> {
            Box::new(self.clone())
        }
    }

    impl ScalarFunctionExecutor for PlusOneFn {
        fn init(
            &mut self,
            args: &Vec<Box<dyn ExpressionExecutor>>,
            _ctx: &Arc<SiddhiAppContext>,
        ) -> Result<(), String> {
            if args.len() != 1 {
                return Err("plusOne expects one argument".to_string());
            }
            self.arg = Some(args[0].clone_executor(_ctx));
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

    let manager = SiddhiManager::new();
    manager.add_scalar_function_factory("plusOne".to_string(), Box::new(PlusOneFn::default()));

    let app_ctx = Arc::new(SiddhiAppContext::new(
        manager.siddhi_context(),
        "app".to_string(),
        Arc::new(SiddhiApp::new("app".to_string())),
        String::new(),
    ));
    let q_ctx = Arc::new(SiddhiQueryContext::new(
        Arc::clone(&app_ctx),
        "Q5".to_string(),
        None,
    ));

    let ctx = ExpressionParserContext {
        siddhi_app_context: app_ctx,
        siddhi_query_context: q_ctx,
        stream_meta_map: HashMap::new(),
        table_meta_map: HashMap::new(),
        window_meta_map: HashMap::new(),
        aggregation_meta_map: HashMap::new(),
        state_meta_map: HashMap::new(),
        default_source: "dummy".to_string(),
        query_name: "Q5",
    };

    let expr = Expression::function_no_ns("plusOne".to_string(), vec![Expression::value_int(4)]);
    let exec = parse_expression(&expr, &ctx).unwrap();
    let res = exec.execute(None);
    assert_eq!(res, Some(AttributeValue::Int(5)));
}

#[test]
fn test_join_query_parsing() {
    let left_def = Arc::new(
        StreamDefinition::new("S1".to_string()).attribute("val".to_string(), AttrType::INT),
    );
    let right_def = Arc::new(
        StreamDefinition::new("S2".to_string()).attribute("val".to_string(), AttrType::INT),
    );

    let left_si = SingleInputStream::new_basic("S1".to_string(), false, false, None, Vec::new());
    let right_si = SingleInputStream::new_basic("S2".to_string(), false, false, None, Vec::new());
    let cond_expr = Expression::compare(
        Expression::Variable(Variable::new("val".to_string()).of_stream("S1".to_string())),
        CompareOp::Equal,
        Expression::Variable(Variable::new("val".to_string()).of_stream("S2".to_string())),
    );
    let input = InputStream::join_stream(
        left_si,
        JoinType::InnerJoin,
        right_si,
        Some(cond_expr.clone()),
        None,
        None,
        None,
    );
    let mut selector = siddhi_rust::query_api::execution::query::selection::Selector::new();
    let insert_action =
        siddhi_rust::query_api::execution::query::output::output_stream::InsertIntoStreamAction {
            target_id: "Out".to_string(),
            is_inner_stream: false,
            is_fault_stream: false,
        };
    let out_stream = siddhi_rust::query_api::execution::query::output::output_stream::OutputStream::new(siddhi_rust::query_api::execution::query::output::output_stream::OutputStreamAction::InsertInto(insert_action), None);
    let query = Query::query()
        .from(input)
        .select(selector)
        .out_stream(out_stream);

    let app_ctx = make_app_ctx();
    let mut junctions = HashMap::new();
    junctions.insert(
        "S1".to_string(),
        Arc::new(std::sync::Mutex::new(
            siddhi_rust::core::stream::stream_junction::StreamJunction::new(
                "S1".to_string(),
                Arc::clone(&left_def),
                Arc::clone(&app_ctx),
                1024,
                false,
                None,
            ),
        )),
    );
    junctions.insert(
        "S2".to_string(),
        Arc::new(std::sync::Mutex::new(
            siddhi_rust::core::stream::stream_junction::StreamJunction::new(
                "S2".to_string(),
                Arc::clone(&right_def),
                Arc::clone(&app_ctx),
                1024,
                false,
                None,
            ),
        )),
    );
    junctions.insert(
        "Out".to_string(),
        Arc::new(std::sync::Mutex::new(
            siddhi_rust::core::stream::stream_junction::StreamJunction::new(
                "Out".to_string(),
                Arc::new(StreamDefinition::new("Out".to_string())),
                Arc::clone(&app_ctx),
                1024,
                false,
                None,
            ),
        )),
    );

    let res = QueryParser::parse_query(
        &query,
        &app_ctx,
        &junctions,
        &HashMap::new(),
        &HashMap::new(),
    );
    assert!(res.is_ok());

    // Also ensure expression parsing works standalone
    let mut left_meta = MetaStreamEvent::new_for_single_input(Arc::clone(&left_def));
    let mut right_meta = MetaStreamEvent::new_for_single_input(Arc::clone(&right_def));
    right_meta.apply_attribute_offset(1);
    let mut map = HashMap::new();
    map.insert("S1".to_string(), Arc::new(left_meta));
    map.insert("S2".to_string(), Arc::new(right_meta));
    let ctx = ExpressionParserContext {
        siddhi_app_context: Arc::clone(&app_ctx),
        siddhi_query_context: make_query_ctx("J"),
        stream_meta_map: map,
        table_meta_map: HashMap::new(),
        window_meta_map: HashMap::new(),
        aggregation_meta_map: HashMap::new(),
        state_meta_map: HashMap::new(),
        default_source: "S1".to_string(),
        query_name: "J",
    };
    let exec = parse_expression(&cond_expr, &ctx).unwrap();
    assert_eq!(exec.get_return_type(), AttrType::BOOL);
}

#[test]
fn test_pattern_query_parsing() {
    let a_def = Arc::new(
        StreamDefinition::new("A".to_string()).attribute("val".to_string(), AttrType::INT),
    );
    let b_def = Arc::new(
        StreamDefinition::new("B".to_string()).attribute("val".to_string(), AttrType::INT),
    );
    let a_si = SingleInputStream::new_basic("A".to_string(), false, false, None, Vec::new());
    let b_si = SingleInputStream::new_basic("B".to_string(), false, false, None, Vec::new());
    let sse1 = State::stream(a_si);
    let sse2 = State::stream(b_si);
    let next = State::next(StateElement::Stream(sse1), StateElement::Stream(sse2));
    let state_stream = StateInputStream::sequence_stream(next, None);
    let input = InputStream::State(Box::new(state_stream));
    let mut selector = siddhi_rust::query_api::execution::query::selection::Selector::new();
    let insert_action =
        siddhi_rust::query_api::execution::query::output::output_stream::InsertIntoStreamAction {
            target_id: "Out".to_string(),
            is_inner_stream: false,
            is_fault_stream: false,
        };
    let out_stream = siddhi_rust::query_api::execution::query::output::output_stream::OutputStream::new(siddhi_rust::query_api::execution::query::output::output_stream::OutputStreamAction::InsertInto(insert_action), None);
    let query = Query::query()
        .from(input)
        .select(selector)
        .out_stream(out_stream);

    let app_ctx = make_app_ctx();
    let mut junctions = HashMap::new();
    junctions.insert(
        "A".to_string(),
        Arc::new(std::sync::Mutex::new(
            siddhi_rust::core::stream::stream_junction::StreamJunction::new(
                "A".to_string(),
                Arc::clone(&a_def),
                Arc::clone(&app_ctx),
                1024,
                false,
                None,
            ),
        )),
    );
    junctions.insert(
        "B".to_string(),
        Arc::new(std::sync::Mutex::new(
            siddhi_rust::core::stream::stream_junction::StreamJunction::new(
                "B".to_string(),
                Arc::clone(&b_def),
                Arc::clone(&app_ctx),
                1024,
                false,
                None,
            ),
        )),
    );
    junctions.insert(
        "Out".to_string(),
        Arc::new(std::sync::Mutex::new(
            siddhi_rust::core::stream::stream_junction::StreamJunction::new(
                "Out".to_string(),
                Arc::new(StreamDefinition::new("Out".to_string())),
                Arc::clone(&app_ctx),
                1024,
                false,
                None,
            ),
        )),
    );

    let res = QueryParser::parse_query(
        &query,
        &app_ctx,
        &junctions,
        &HashMap::new(),
        &HashMap::new(),
    );
    assert!(res.is_ok());
}

#[test]
fn test_table_in_expression_query() {
    use siddhi_rust::core::table::{InMemoryTable, Table};
    use siddhi_rust::query_api::definition::TableDefinition;

    let s_def = Arc::new(
        StreamDefinition::new("S".to_string()).attribute("val".to_string(), AttrType::INT),
    );
    let t_def =
        Arc::new(TableDefinition::new("T".to_string()).attribute("val".to_string(), AttrType::INT));
    let s_si = SingleInputStream::new_basic("S".to_string(), false, false, None, Vec::new());
    let filter = Expression::in_op(
        Expression::Variable(Variable::new("val".to_string()).of_stream("S".to_string())),
        "T".to_string(),
    );
    let filtered = s_si.filter(filter);
    let input = InputStream::Single(filtered);
    let selector = siddhi_rust::query_api::execution::query::selection::Selector::new();
    let insert_action =
        siddhi_rust::query_api::execution::query::output::output_stream::InsertIntoStreamAction {
            target_id: "Out".to_string(),
            is_inner_stream: false,
            is_fault_stream: false,
        };
    let out_stream = siddhi_rust::query_api::execution::query::output::output_stream::OutputStream::new(siddhi_rust::query_api::execution::query::output::output_stream::OutputStreamAction::InsertInto(insert_action), None);
    let query = Query::query()
        .from(input)
        .select(selector)
        .out_stream(out_stream);

    let app_ctx = make_app_ctx();
    let table: Arc<dyn Table> = Arc::new(InMemoryTable::new());
    app_ctx
        .get_siddhi_context()
        .add_table("T".to_string(), table);
    let mut junctions = HashMap::new();
    junctions.insert(
        "S".to_string(),
        Arc::new(std::sync::Mutex::new(
            siddhi_rust::core::stream::stream_junction::StreamJunction::new(
                "S".to_string(),
                Arc::clone(&s_def),
                Arc::clone(&app_ctx),
                1024,
                false,
                None,
            ),
        )),
    );
    junctions.insert(
        "Out".to_string(),
        Arc::new(std::sync::Mutex::new(
            siddhi_rust::core::stream::stream_junction::StreamJunction::new(
                "Out".to_string(),
                Arc::new(StreamDefinition::new("Out".to_string())),
                Arc::clone(&app_ctx),
                1024,
                false,
                None,
            ),
        )),
    );
    let mut table_defs = HashMap::new();
    table_defs.insert("T".to_string(), t_def);

    let res = QueryParser::parse_query(&query, &app_ctx, &junctions, &table_defs, &HashMap::new());
    assert!(res.is_ok());
}
#[test]
fn test_join_query_parsing_from_string() {
    use siddhi_rust::query_compiler::{parse_query, parse_stream_definition};
    use std::sync::Mutex;
    let s1_def = Arc::new(parse_stream_definition("define stream S1 (val int)").unwrap());
    let s2_def = Arc::new(parse_stream_definition("define stream S2 (val int)").unwrap());
    let query =
        parse_query("from S1 join S2 on S1.val == S2.val select S1.val, S2.val insert into Out")
            .unwrap();
    let app_ctx = make_app_ctx();
    let mut junctions = HashMap::new();
    junctions.insert(
        "S1".to_string(),
        Arc::new(Mutex::new(
            siddhi_rust::core::stream::stream_junction::StreamJunction::new(
                "S1".to_string(),
                Arc::clone(&s1_def),
                Arc::clone(&app_ctx),
                1024,
                false,
                None,
            ),
        )),
    );
    junctions.insert(
        "S2".to_string(),
        Arc::new(Mutex::new(
            siddhi_rust::core::stream::stream_junction::StreamJunction::new(
                "S2".to_string(),
                Arc::clone(&s2_def),
                Arc::clone(&app_ctx),
                1024,
                false,
                None,
            ),
        )),
    );
    junctions.insert(
        "Out".to_string(),
        Arc::new(Mutex::new(
            siddhi_rust::core::stream::stream_junction::StreamJunction::new(
                "Out".to_string(),
                Arc::new(StreamDefinition::new("Out".to_string())),
                Arc::clone(&app_ctx),
                1024,
                false,
                None,
            ),
        )),
    );
    let res = QueryParser::parse_query(
        &query,
        &app_ctx,
        &junctions,
        &HashMap::new(),
        &HashMap::new(),
    );
    assert!(res.is_ok());
}

#[test]
fn test_pattern_query_parsing_from_string() {
    use siddhi_rust::query_compiler::{parse_query, parse_stream_definition};
    use std::sync::Mutex;
    let a_def = Arc::new(parse_stream_definition("define stream A (val int)").unwrap());
    let b_def = Arc::new(parse_stream_definition("define stream B (val int)").unwrap());
    let query = parse_query("from A -> B select A.val, B.val insert into Out").unwrap();
    let app_ctx = make_app_ctx();
    let mut junctions = HashMap::new();
    junctions.insert(
        "A".to_string(),
        Arc::new(Mutex::new(
            siddhi_rust::core::stream::stream_junction::StreamJunction::new(
                "A".to_string(),
                Arc::clone(&a_def),
                Arc::clone(&app_ctx),
                1024,
                false,
                None,
            ),
        )),
    );
    junctions.insert(
        "B".to_string(),
        Arc::new(Mutex::new(
            siddhi_rust::core::stream::stream_junction::StreamJunction::new(
                "B".to_string(),
                Arc::clone(&b_def),
                Arc::clone(&app_ctx),
                1024,
                false,
                None,
            ),
        )),
    );
    junctions.insert(
        "Out".to_string(),
        Arc::new(Mutex::new(
            siddhi_rust::core::stream::stream_junction::StreamJunction::new(
                "Out".to_string(),
                Arc::new(StreamDefinition::new("Out".to_string())),
                Arc::clone(&app_ctx),
                1024,
                false,
                None,
            ),
        )),
    );
    let res = QueryParser::parse_query(
        &query,
        &app_ctx,
        &junctions,
        &HashMap::new(),
        &HashMap::new(),
    );
    assert!(res.is_ok());
}
