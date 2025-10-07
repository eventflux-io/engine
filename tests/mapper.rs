#[path = "common/mod.rs"]
mod common;
use common::AppRunner;

use eventflux_rust::core::event::event::Event;
use eventflux_rust::core::event::value::AttributeValue;
use eventflux_rust::core::stream::input::mapper::SourceMapper;
use eventflux_rust::core::stream::output::mapper::SinkMapper;

#[derive(Debug, Clone)]
struct CsvSourceMapper;

impl SourceMapper for CsvSourceMapper {
    fn map(&self, input: &[u8]) -> Vec<Event> {
        let text = std::str::from_utf8(input).unwrap();
        text.lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                let values: Vec<AttributeValue> = l
                    .split(',')
                    .map(|p| AttributeValue::Int(p.trim().parse().unwrap()))
                    .collect();
                Event::new_with_data(0, values)
            })
            .collect()
    }

    fn clone_box(&self) -> Box<dyn SourceMapper> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
struct ConcatSinkMapper;

impl SinkMapper for ConcatSinkMapper {
    fn map(&self, events: &[Event]) -> Vec<u8> {
        let mut parts = Vec::new();
        for e in events {
            for v in &e.data {
                if let AttributeValue::Int(i) = v {
                    parts.push(i.to_string());
                }
            }
        }
        parts.join(";").into_bytes()
    }

    fn clone_box(&self) -> Box<dyn SinkMapper> {
        Box::new(self.clone())
    }
}

// TODO: NOT PART OF M1 - Old EventFluxQL syntax
// This test uses "define stream" and old query syntax which is not supported by SQL parser.
// Source/sink mappers functionality exists but SQL syntax for them is not part of M1.
// See feat/grammar/GRAMMAR_STATUS.md for M1 feature list.
#[tokio::test]
#[ignore = "Old EventFluxQL syntax not part of M1"]
async fn test_source_and_sink_mapper_usage() {
    let app = "\
        define stream In (a int, b int);\n\
        define stream Out (a int, b int);\n\
        from In select a, b insert into Out;\n";
    let runner = AppRunner::new(app, "Out").await;

    let src_mapper = CsvSourceMapper;
    let events = src_mapper.map(b"1,2\n3,4");
    let data: Vec<Vec<AttributeValue>> = events.into_iter().map(|e| e.data).collect();
    runner.send_batch("In", data);

    let results = runner.shutdown();
    let out_events: Vec<Event> = results
        .iter()
        .map(|row| Event::new_with_data(0, row.clone()))
        .collect();
    let sink_mapper = ConcatSinkMapper;
    let bytes = sink_mapper.map(&out_events);
    assert_eq!(bytes, b"1;2;3;4".to_vec());
}
