// SPDX-License-Identifier: MIT OR Apache-2.0

use eventflux_rust::core::config::eventflux_app_context::EventFluxAppContext;
use eventflux_rust::core::config::eventflux_context::EventFluxContext;
use eventflux_rust::core::config::eventflux_query_context::EventFluxQueryContext;
use eventflux_rust::core::event::complex_event::ComplexEvent;
use eventflux_rust::core::event::event::Event;
use eventflux_rust::core::event::stream::stream_event::StreamEvent;
use eventflux_rust::core::event::value::AttributeValue;
use eventflux_rust::core::persistence::data_source::{DataSource, DataSourceConfig};
use eventflux_rust::core::query::output::InsertIntoTableProcessor;
use eventflux_rust::core::query::processor::Processor;
use eventflux_rust::core::stream::input::table_input_handler::TableInputHandler;
use eventflux_rust::core::table::JdbcTable;
use eventflux_rust::core::table::{InMemoryCompiledCondition, InMemoryTable, Table};
use rusqlite::Connection;
use std::any::Any;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Debug)]
struct SqliteDataSource {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteDataSource {
    fn new(path: &str) -> Self {
        Self {
            conn: Arc::new(Mutex::new(Connection::open(path).unwrap())),
        }
    }
}

impl DataSource for SqliteDataSource {
    fn get_type(&self) -> String {
        "sqlite".to_string()
    }
    fn init(
        &mut self,
        _ctx: &Arc<EventFluxAppContext>,
        _id: &str,
        _cfg: DataSourceConfig,
    ) -> Result<(), String> {
        Ok(())
    }
    fn get_connection(&self) -> Result<Box<dyn Any>, String> {
        Ok(Box::new(self.conn.clone()) as Box<dyn Any>)
    }
    fn shutdown(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn clone_data_source(&self) -> Box<dyn DataSource> {
        Box::new(SqliteDataSource {
            conn: self.conn.clone(),
        })
    }
}

fn setup_jdbc_table(ctx: &Arc<EventFluxContext>, table_name: &str) {
    let ds = ctx.get_data_source("DS1").unwrap();
    let conn_any = ds.get_connection().unwrap();
    let conn_arc = conn_any.downcast::<Arc<Mutex<Connection>>>().unwrap();
    let conn = conn_arc.lock().unwrap();
    let sql = format!("CREATE TABLE {table_name} (c0 TEXT)");
    conn.execute(&sql, []).unwrap();
}

#[test]
fn test_table_input_handler_add() {
    let ctx = Arc::new(EventFluxAppContext::new(
        Arc::new(EventFluxContext::default()),
        "app".to_string(),
        Arc::new(eventflux_rust::query_api::eventflux_app::EventFluxApp::new(
            "app".to_string(),
        )),
        String::new(),
    ));
    let table: Arc<dyn Table> = Arc::new(InMemoryTable::new());
    ctx.get_eventflux_context()
        .add_table("T1".to_string(), table.clone());
    let handler = TableInputHandler::new(table, Arc::clone(&ctx));
    handler.add(vec![Event::new_with_data(0, vec![AttributeValue::Int(5)])]);
    assert!(ctx
        .get_eventflux_context()
        .get_table("T1")
        .unwrap()
        .contains(&InMemoryCompiledCondition {
            values: vec![AttributeValue::Int(5)]
        }));
}

#[test]
fn test_insert_into_table_processor() {
    let app_ctx = Arc::new(EventFluxAppContext::new(
        Arc::new(EventFluxContext::default()),
        "app".to_string(),
        Arc::new(eventflux_rust::query_api::eventflux_app::EventFluxApp::new(
            "app".to_string(),
        )),
        String::new(),
    ));
    let table: Arc<dyn Table> = Arc::new(InMemoryTable::new());
    app_ctx
        .get_eventflux_context()
        .add_table("T2".to_string(), table.clone());
    let query_ctx = Arc::new(EventFluxQueryContext::new(
        Arc::clone(&app_ctx),
        "q".to_string(),
        None,
    ));
    let proc = InsertIntoTableProcessor::new(table, Arc::clone(&app_ctx), Arc::clone(&query_ctx));
    let mut se = StreamEvent::new(0, 0, 0, 1);
    se.set_output_data(Some(vec![AttributeValue::Int(7)]));
    proc.process(Some(Box::new(se)));
    assert!(app_ctx
        .get_eventflux_context()
        .get_table("T2")
        .unwrap()
        .contains(&InMemoryCompiledCondition {
            values: vec![AttributeValue::Int(7)]
        }));
}

#[test]
fn test_table_input_handler_update_delete_find() {
    let ctx = Arc::new(EventFluxAppContext::new(
        Arc::new(EventFluxContext::default()),
        "app".to_string(),
        Arc::new(eventflux_rust::query_api::eventflux_app::EventFluxApp::new(
            "app".to_string(),
        )),
        String::new(),
    ));
    let table: Arc<dyn Table> = Arc::new(InMemoryTable::new());
    ctx.get_eventflux_context()
        .add_table("T3".to_string(), table.clone());
    let handler = TableInputHandler::new(table.clone(), Arc::clone(&ctx));
    handler.add(vec![Event::new_with_data(
        0,
        vec![AttributeValue::String("a".into())],
    )]);
    assert!(table.contains(&InMemoryCompiledCondition {
        values: vec![AttributeValue::String("a".into())]
    }));
    assert!(handler.update(
        vec![AttributeValue::String("a".into())],
        vec![AttributeValue::String("b".into())]
    ));
    assert!(table.contains(&InMemoryCompiledCondition {
        values: vec![AttributeValue::String("b".into())]
    }));
    assert!(handler.delete(vec![AttributeValue::String("b".into())]));
    assert!(!table.contains(&InMemoryCompiledCondition {
        values: vec![AttributeValue::String("b".into())]
    }));
    handler.add(vec![Event::new_with_data(
        0,
        vec![AttributeValue::String("x".into())],
    )]);
    assert_eq!(
        table.find(&InMemoryCompiledCondition {
            values: vec![AttributeValue::String("x".into())]
        }),
        Some(vec![AttributeValue::String("x".into())])
    );
}

#[test]
fn test_table_input_handler_jdbc() {
    let ctx = Arc::new(EventFluxContext::new());
    ctx.add_data_source(
        "DS1".to_string(),
        Arc::new(SqliteDataSource::new(":memory:")),
    )
    .unwrap();
    setup_jdbc_table(&ctx, "test2");

    let app_ctx = Arc::new(EventFluxAppContext::new(
        Arc::clone(&ctx),
        "app".to_string(),
        Arc::new(eventflux_rust::query_api::eventflux_app::EventFluxApp::new(
            "app".to_string(),
        )),
        String::new(),
    ));
    let table: Arc<dyn Table> =
        Arc::new(JdbcTable::new("test2".to_string(), "DS1".to_string(), Arc::clone(&ctx)).unwrap());
    app_ctx
        .get_eventflux_context()
        .add_table("J1".to_string(), table.clone());
    let handler = TableInputHandler::new(table.clone(), Arc::clone(&app_ctx));

    handler.add(vec![Event::new_with_data(
        0,
        vec![AttributeValue::String("a".into())],
    )]);
    assert!(table.contains(&InMemoryCompiledCondition {
        values: vec![AttributeValue::String("a".into())]
    }));
    assert!(handler.update(
        vec![AttributeValue::String("a".into())],
        vec![AttributeValue::String("b".into())]
    ));
    assert!(table.contains(&InMemoryCompiledCondition {
        values: vec![AttributeValue::String("b".into())]
    }));
    assert!(handler.delete(vec![AttributeValue::String("b".into())]));
    assert!(!table.contains(&InMemoryCompiledCondition {
        values: vec![AttributeValue::String("b".into())]
    }));
}

#[test]
fn test_query_parser_with_table_actions() {
    use eventflux_rust::core::stream::stream_junction::StreamJunction;
    use eventflux_rust::query_compiler::{parse_query, parse_stream_definition};
    use std::collections::HashMap;
    use std::sync::Mutex;

    let s_def = Arc::new(parse_stream_definition("define stream S (val string)").unwrap());
    let t_def = Arc::new(
        eventflux_rust::query_compiler::parse_table_definition("define table T (val string)")
            .unwrap(),
    );
    let app_ctx = Arc::new(EventFluxAppContext::new(
        Arc::new(EventFluxContext::default()),
        "app".to_string(),
        Arc::new(eventflux_rust::query_api::eventflux_app::EventFluxApp::new(
            "app".to_string(),
        )),
        String::new(),
    ));
    app_ctx
        .get_eventflux_context()
        .add_table("T".to_string(), Arc::new(InMemoryTable::new()));

    let mut junctions = HashMap::new();
    junctions.insert(
        "S".to_string(),
        Arc::new(Mutex::new(StreamJunction::new(
            "S".to_string(),
            Arc::clone(&s_def),
            Arc::clone(&app_ctx),
            1024,
            false,
            None,
        ))),
    );

    let mut table_defs = HashMap::new();
    table_defs.insert("T".to_string(), t_def);

    let q1 = parse_query("from S select val insert into table T").unwrap();
    assert!(
        eventflux_rust::core::util::parser::QueryParser::parse_query(
            &q1,
            &app_ctx,
            &junctions,
            &table_defs,
            &HashMap::new(),
            None
        )
        .is_ok()
    );

    let q2 = parse_query("from S select val update table T").unwrap();
    assert!(
        eventflux_rust::core::util::parser::QueryParser::parse_query(
            &q2,
            &app_ctx,
            &junctions,
            &table_defs,
            &HashMap::new(),
            None
        )
        .is_ok()
    );

    let q3 = parse_query("from S select val delete table T").unwrap();
    assert!(
        eventflux_rust::core::util::parser::QueryParser::parse_query(
            &q3,
            &app_ctx,
            &junctions,
            &table_defs,
            &HashMap::new(),
            None
        )
        .is_ok()
    );
}
