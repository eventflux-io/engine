// siddhi_rust/src/core/util/parser/query_parser.rs
use super::expression_parser::{parse_expression, ExpressionParserContext};
use crate::core::config::siddhi_app_context::SiddhiAppContext;
use crate::core::config::siddhi_query_context::SiddhiQueryContext;
use crate::core::event::stream::meta_stream_event::MetaStreamEvent;
use crate::core::query::input::stream::join::{JoinProcessor, JoinProcessorSide, JoinSide};
use crate::core::query::input::stream::state::{SequenceProcessor, SequenceSide, SequenceType};
use crate::core::query::output::insert_into_aggregation_processor::InsertIntoAggregationProcessor;
use crate::core::query::output::insert_into_stream_processor::InsertIntoStreamProcessor;
use crate::core::query::processor::stream::filter::FilterProcessor;
use crate::core::query::processor::stream::window::create_window_processor;
use crate::core::query::processor::Processor; // Trait
use crate::core::query::query_runtime::QueryRuntime;
use crate::core::query::selector::attribute::OutputAttributeProcessor; // OAP
use crate::core::query::selector::select_processor::{OutputRateLimiter, SelectProcessor};
use crate::core::query::selector::{GroupByKeyGenerator, OrderByEventComparator};
use crate::core::stream::stream_junction::StreamJunction;
use crate::query_api::{
    definition::Attribute as ApiAttribute, // For constructing output attributes
    definition::StreamDefinition as ApiStreamDefinition,
    execution::query::input::InputStream as ApiInputStream,
    execution::query::output::OutputEventType as ApiOutputEventType,
    execution::query::output::OutputStream as ApiOutputStream,
    // Import other query_api parts as needed (Selector, OutputAttribute, OutputStream)
    execution::query::selection::OutputAttribute as ApiOutputAttribute,
    execution::query::Query as ApiQuery,
    expression::Expression as ApiExpression, // Added this import
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
// use rand; // For generating random names, if not using query name from annotation

pub struct QueryParser;

impl QueryParser {
    pub fn parse_query(
        api_query: &ApiQuery,
        siddhi_app_context: &Arc<SiddhiAppContext>,
        stream_junction_map: &HashMap<String, Arc<Mutex<StreamJunction>>>,
        table_def_map: &HashMap<String, Arc<crate::query_api::definition::TableDefinition>>,
        aggregation_map: &HashMap<String, Arc<Mutex<crate::core::aggregation::AggregationRuntime>>>,
    ) -> Result<QueryRuntime, String> {
        // 1. Determine Query Name (from @info(name='foo') or generate)
        let query_name = api_query
            .annotations
            .iter()
            .find(|ann| ann.name == "info")
            .and_then(|ann| ann.elements.iter().find(|el| el.key == "name"))
            .map(|el| el.value.clone())
            .unwrap_or_else(|| format!("query_{}", uuid::Uuid::new_v4().hyphenated()));

        let siddhi_query_context = Arc::new(SiddhiQueryContext::new(
            Arc::clone(siddhi_app_context),
            query_name.clone(),
            None, // partition_id, assuming not in a partition for now
        ));

        // 2. Identify input stream & get its junction
        let input_stream_api = api_query
            .input_stream
            .as_ref()
            .ok_or_else(|| format!("Query '{}' has no input stream defined.", query_name))?;

        let mut processor_chain_head: Option<Arc<Mutex<dyn Processor>>> = None;
        let mut last_processor_in_chain: Option<Arc<Mutex<dyn Processor>>> = None;

        // Helper closure to link processors
        let mut link_processor = |new_processor_arc: Arc<Mutex<dyn Processor>>| {
            if processor_chain_head.is_none() {
                processor_chain_head = Some(new_processor_arc.clone());
            }
            if let Some(ref last_p_arc) = last_processor_in_chain {
                last_p_arc
                    .lock()
                    .expect("Processor Mutex poisoned")
                    .set_next_processor(Some(new_processor_arc.clone()));
            }
            last_processor_in_chain = Some(new_processor_arc);
        };

        // Build metadata and input processors depending on stream type
        let expr_parser_context: ExpressionParserContext = match input_stream_api {
            ApiInputStream::Single(single_in_stream) => {
                let input_stream_id = single_in_stream.get_stream_id_str().to_string();
                let input_junction = stream_junction_map
                    .get(&input_stream_id)
                    .ok_or_else(|| {
                        format!(
                            "Input stream '{}' not found for query '{}'",
                            input_stream_id, query_name
                        )
                    })?
                    .clone();
                let input_stream_def_from_junction = input_junction
                    .lock()
                    .expect("Input junction Mutex poisoned")
                    .get_stream_definition();
                let meta_input_event = Arc::new(MetaStreamEvent::new_for_single_input(
                    input_stream_def_from_junction,
                ));
                let mut stream_meta_map = HashMap::new();
                stream_meta_map.insert(input_stream_id.clone(), Arc::clone(&meta_input_event));
                // Table metadata is only required when a table participates as an
                // input source. Since table queries are not yet supported, avoid
                // registering tables here to prevent variable lookup ambiguity.
                let table_meta_map = HashMap::new();
                let ctx = ExpressionParserContext {
                    siddhi_app_context: Arc::clone(siddhi_app_context),
                    siddhi_query_context: Arc::clone(&siddhi_query_context),
                    stream_meta_map,
                    table_meta_map,
                    window_meta_map: HashMap::new(),
                    aggregation_meta_map: HashMap::new(),
                    state_meta_map: HashMap::new(),
                    stream_positions: {
                        let mut m = HashMap::new();
                        m.insert(input_stream_id.clone(), 0);
                        m
                    },
                    default_source: input_stream_id.clone(),
                    query_name: &query_name,
                };

                for handler in single_in_stream.get_stream_handlers() {
                    match handler {
                        crate::query_api::execution::query::input::handler::StreamHandler::Window(w) => {
                            let win_proc = create_window_processor(
                                w.as_ref(),
                                Arc::clone(siddhi_app_context),
                                Arc::clone(&siddhi_query_context),
                            )?;
                            link_processor(win_proc);
                        }
                        crate::query_api::execution::query::input::handler::StreamHandler::Filter(f) => {
                            let condition_executor =
                                parse_expression(&f.filter_expression, &ctx).map_err(|e| e.to_string())?;
                            let filter_processor = Arc::new(Mutex::new(FilterProcessor::new(
                                condition_executor,
                                Arc::clone(siddhi_app_context),
                                Arc::clone(&siddhi_query_context),
                            )?));
                            link_processor(filter_processor);
                        }
                        _ => {}
                    }
                }

                ctx
            }
            ApiInputStream::Join(join_stream) => {
                use crate::query_api::execution::query::input::stream::join_input_stream::JoinInputStream as ApiJoin;
                let left_id = join_stream
                    .left_input_stream
                    .get_stream_id_str()
                    .to_string();
                let right_id = join_stream
                    .right_input_stream
                    .get_stream_id_str()
                    .to_string();
                let left_junction = stream_junction_map
                    .get(&left_id)
                    .ok_or_else(|| format!("Input stream '{}' not found", left_id))?
                    .clone();
                let right_junction = stream_junction_map
                    .get(&right_id)
                    .ok_or_else(|| format!("Input stream '{}' not found", right_id))?
                    .clone();

                let left_def = left_junction.lock().unwrap().get_stream_definition();
                let right_def = right_junction.lock().unwrap().get_stream_definition();
                let left_len = left_def.abstract_definition.attribute_list.len();
                let right_len = right_def.abstract_definition.attribute_list.len();

                let mut left_meta = MetaStreamEvent::new_for_single_input(left_def);
                let mut right_meta = MetaStreamEvent::new_for_single_input(right_def);
                right_meta.apply_attribute_offset(left_len);

                let mut stream_meta_map = HashMap::new();
                stream_meta_map.insert(left_id.clone(), Arc::new(left_meta));
                stream_meta_map.insert(right_id.clone(), Arc::new(right_meta));

                // Table metadata is irrelevant for join parsing until table
                // sources are supported. Leaving the map empty avoids
                // incorrectly flagging attribute ambiguities.
                let table_meta_map = HashMap::new();

                let cond_exec = if let Some(expr) = &join_stream.on_compare {
                    Some(
                        parse_expression(
                            expr,
                            &ExpressionParserContext {
                                siddhi_app_context: Arc::clone(siddhi_app_context),
                                siddhi_query_context: Arc::clone(&siddhi_query_context),
                                stream_meta_map: stream_meta_map.clone(),
                                table_meta_map: table_meta_map.clone(),
                                window_meta_map: HashMap::new(),
                                aggregation_meta_map: HashMap::new(),
                                state_meta_map: HashMap::new(),
                                stream_positions: {
                                    let mut m = HashMap::new();
                                    m.insert(left_id.clone(), 0);
                                    m.insert(right_id.clone(), 1);
                                    m
                                },
                                default_source: left_id.clone(),
                                query_name: &query_name,
                            },
                        )
                        .map_err(|e| e.to_string())?,
                    )
                } else {
                    None
                };

                let join_proc = Arc::new(Mutex::new(JoinProcessor::new(
                    join_stream.join_type,
                    cond_exec,
                    left_len,
                    right_len,
                    Arc::clone(siddhi_app_context),
                    Arc::clone(&siddhi_query_context),
                )));
                let left_side = JoinProcessor::create_side_processor(&join_proc, JoinSide::Left);
                let right_side = JoinProcessor::create_side_processor(&join_proc, JoinSide::Right);

                left_junction.lock().unwrap().subscribe(left_side.clone());
                right_junction.lock().unwrap().subscribe(right_side.clone());

                // Treat the left side processor as the entry point for the
                // processor chain so that downstream processors can be linked
                // using the existing `link_processor` helper.
                link_processor(left_side.clone());

                // The right side is subscribed directly and shares the same
                // JoinProcessor through `join_proc`, so it does not need to be
                // part of the sequential chain.

                let ctx = ExpressionParserContext {
                    siddhi_app_context: Arc::clone(siddhi_app_context),
                    siddhi_query_context: Arc::clone(&siddhi_query_context),
                    stream_meta_map,
                    table_meta_map,
                    window_meta_map: HashMap::new(),
                    aggregation_meta_map: HashMap::new(),
                    state_meta_map: HashMap::new(),
                    stream_positions: {
                        let mut m = HashMap::new();
                        m.insert(left_id.clone(), 0);
                        m.insert(right_id.clone(), 1);
                        m
                    },
                    default_source: left_id.clone(),
                    query_name: &query_name,
                };
                ctx
            }
            ApiInputStream::State(state_stream) => {
                use crate::core::query::input::stream::state::{
                    LogicalProcessor, LogicalType, SequenceProcessor, SequenceSide, SequenceType,
                };
                use crate::query_api::execution::query::input::state::logical_state_element::Type as ApiLogicalType;
                use crate::query_api::execution::query::input::state::state_element::StateElement;

                fn extract_stream_state_with_count<'a>(
                    se: &'a StateElement,
                ) -> Option<(
                    &'a crate::query_api::execution::query::input::state::stream_state_element::StreamStateElement,
                    i32,
                    i32,
                )> {
                    match se {
                        StateElement::Stream(s) => Some((s, 1, 1)),
                        StateElement::Every(ev) =>
                            extract_stream_state_with_count(&ev.state_element),
                        StateElement::Count(c) => Some((
                            &c.stream_state_element,
                            c.min_count,
                            c.max_count,
                        )),
                        _ => None,
                    }
                }

                enum StateRuntimeKind {
                    Sequence {
                        first_id: String,
                        second_id: String,
                        seq_type: SequenceType,
                        first_min: i32,
                        first_max: i32,
                        second_min: i32,
                        second_max: i32,
                        within: Option<i64>,
                    },
                    Logical {
                        first_id: String,
                        second_id: String,
                        logical_type: LogicalType,
                    },
                }

                let runtime_kind = match state_stream.state_element.as_ref() {
                    StateElement::Next(next_elem) => {
                        let (s1, fmin, fmax) =
                            extract_stream_state_with_count(&next_elem.state_element).ok_or_else(|| {
                                format!(
                                    "Query '{}': Unsupported Next pattern structure",
                                    query_name
                                )
                            })?;
                        let (s2, smin, smax) =
                            extract_stream_state_with_count(&next_elem.next_state_element).ok_or_else(
                                || {
                                    format!(
                                        "Query '{}': Unsupported Next pattern structure",
                                        query_name
                                    )
                                },
                            )?;
                        let seq_type = match state_stream.state_type {
                            crate::query_api::execution::query::input::stream::state_input_stream::Type::Pattern => SequenceType::Pattern,
                            crate::query_api::execution::query::input::stream::state_input_stream::Type::Sequence => SequenceType::Sequence,
                        };
                        let within = state_stream
                            .within_time
                            .as_ref()
                            .and_then(|c| match c.get_value() {
                                crate::query_api::expression::constant::ConstantValueWithFloat::Time(t) => Some(*t),
                                crate::query_api::expression::constant::ConstantValueWithFloat::Long(l) => Some(*l),
                                crate::query_api::expression::constant::ConstantValueWithFloat::Int(i) => Some(*i as i64),
                                _ => None,
                            });
                        StateRuntimeKind::Sequence {
                            first_id: s1.get_single_input_stream().get_stream_id_str().to_string(),
                            second_id: s2.get_single_input_stream().get_stream_id_str().to_string(),
                            seq_type,
                            first_min: fmin,
                            first_max: fmax,
                            second_min: smin,
                            second_max: smax,
                            within,
                        }
                    }
                    StateElement::Logical(log_elem) => {
                        let (s1, _, _) =
                            extract_stream_state_with_count(&log_elem.stream_state_element_1)
                            .ok_or_else(|| {
                                format!(
                                    "Query '{}': Unsupported Logical pattern structure",
                                    query_name
                                )
                            })?;
                        let (s2, _, _) =
                            extract_stream_state_with_count(&log_elem.stream_state_element_2)
                            .ok_or_else(|| {
                                format!(
                                    "Query '{}': Unsupported Logical pattern structure",
                                    query_name
                                )
                            })?;
                        let logical_type = match log_elem.logical_type {
                            ApiLogicalType::And => LogicalType::And,
                            ApiLogicalType::Or => LogicalType::Or,
                        };
                        StateRuntimeKind::Logical {
                            first_id: s1.get_single_input_stream().get_stream_id_str().to_string(),
                            second_id: s2.get_single_input_stream().get_stream_id_str().to_string(),
                            logical_type,
                        }
                    }
                    _ => {
                        return Err(format!("Query '{}': Unsupported state element", query_name));
                    }
                };

                let (
                    first_junction,
                    second_junction,
                    first_len,
                    second_len,
                    mut stream_meta_map,
                    first_id_clone,
                    second_id_clone,
                ) = {
                    let fid_str = match &runtime_kind {
                        StateRuntimeKind::Sequence { first_id, .. }
                        | StateRuntimeKind::Logical { first_id, .. } => first_id,
                    };
                    let sid_str = match &runtime_kind {
                        StateRuntimeKind::Sequence { second_id, .. }
                        | StateRuntimeKind::Logical { second_id, .. } => second_id,
                    };

                    let first_junction = stream_junction_map
                        .get(fid_str)
                        .ok_or_else(|| format!("Input stream '{}' not found", fid_str))?
                        .clone();
                    let second_junction = stream_junction_map
                        .get(sid_str)
                        .ok_or_else(|| format!("Input stream '{}' not found", sid_str))?
                        .clone();

                    let first_def = first_junction.lock().unwrap().get_stream_definition();
                    let second_def = second_junction.lock().unwrap().get_stream_definition();
                    let first_len = first_def.abstract_definition.attribute_list.len();
                    let second_len = second_def.abstract_definition.attribute_list.len();

                    let mut first_meta = MetaStreamEvent::new_for_single_input(first_def);
                    let mut second_meta = MetaStreamEvent::new_for_single_input(second_def);
                    second_meta.apply_attribute_offset(first_len);
                    let mut stream_meta_map = HashMap::new();
                    stream_meta_map.insert(fid_str.clone(), Arc::new(first_meta));
                    stream_meta_map.insert(sid_str.clone(), Arc::new(second_meta));
                    (
                        first_junction,
                        second_junction,
                        first_len,
                        second_len,
                        stream_meta_map,
                        fid_str.clone(),
                        sid_str.clone(),
                    )
                };

                // Tables are not currently supported as inputs for state
                // streams, so avoid creating metadata entries that would clash
                // with the actual stream attributes.
                let table_meta_map = HashMap::new();

                match runtime_kind {
                    StateRuntimeKind::Sequence {
                        first_id: fid,
                        second_id: sid,
                        seq_type,
                        first_min,
                        first_max,
                        second_min,
                        second_max,
                        within,
                    } => {
                        let seq_proc = Arc::new(Mutex::new(SequenceProcessor::new(
                            seq_type,
                            first_len,
                            second_len,
                            first_min,
                            first_max,
                            second_min,
                            second_max,
                            within,
                            Arc::clone(siddhi_app_context),
                            Arc::clone(&siddhi_query_context),
                        )));
                        let first_side = SequenceProcessor::create_side_processor(
                            &seq_proc,
                            SequenceSide::First,
                        );
                        let second_side = SequenceProcessor::create_side_processor(
                            &seq_proc,
                            SequenceSide::Second,
                        );

                        first_junction.lock().unwrap().subscribe(first_side.clone());
                        second_junction
                            .lock()
                            .unwrap()
                            .subscribe(second_side.clone());

                        link_processor(first_side.clone());

                        ExpressionParserContext {
                            siddhi_app_context: Arc::clone(siddhi_app_context),
                            siddhi_query_context: Arc::clone(&siddhi_query_context),
                            stream_meta_map,
                            table_meta_map,
                            window_meta_map: HashMap::new(),
                            aggregation_meta_map: HashMap::new(),
                            state_meta_map: HashMap::new(),
                            stream_positions: {
                                let mut m = HashMap::new();
                                m.insert(first_id_clone.clone(), 0);
                                m.insert(second_id_clone.clone(), 1);
                                m
                            },
                            default_source: first_id_clone.clone(),
                            query_name: &query_name,
                        }
                    }
                    StateRuntimeKind::Logical {
                        first_id: fid,
                        second_id: sid,
                        logical_type,
                    } => {
                        let log_proc = Arc::new(Mutex::new(LogicalProcessor::new(
                            logical_type,
                            first_len,
                            second_len,
                            Arc::clone(siddhi_app_context),
                            Arc::clone(&siddhi_query_context),
                        )));
                        let first_side =
                            LogicalProcessor::create_side_processor(&log_proc, SequenceSide::First);
                        let second_side = LogicalProcessor::create_side_processor(
                            &log_proc,
                            SequenceSide::Second,
                        );

                        first_junction.lock().unwrap().subscribe(first_side.clone());
                        second_junction
                            .lock()
                            .unwrap()
                            .subscribe(second_side.clone());

                        link_processor(first_side.clone());

                        ExpressionParserContext {
                            siddhi_app_context: Arc::clone(siddhi_app_context),
                            siddhi_query_context: Arc::clone(&siddhi_query_context),
                            stream_meta_map,
                            table_meta_map,
                            window_meta_map: HashMap::new(),
                            aggregation_meta_map: HashMap::new(),
                            state_meta_map: HashMap::new(),
                            stream_positions: {
                                let mut m = HashMap::new();
                                m.insert(first_id_clone.clone(), 0);
                                m.insert(second_id_clone.clone(), 1);
                                m
                            },
                            default_source: first_id_clone.clone(),
                            query_name: &query_name,
                        }
                    }
                }

                // unreachable
            }
            _ => {
                return Err(format!(
                    "Query '{}': Unsupported input stream type",
                    query_name
                ));
            }
        };

        // 5. Selector (Projections)
        let api_selector = &api_query.selector; // Selector is not Option in query_api::Query
        let mut oaps = Vec::new();
        let mut output_attributes_for_def = Vec::new();

        for (idx, api_out_attr) in api_selector.selection_list.iter().enumerate() {
            let expr_exec =
                parse_expression(&api_out_attr.expression, &expr_parser_context).map_err(|e| e.to_string())?;
            // OutputAttributeProcessor needs the output position.
            let oap = OutputAttributeProcessor::new(expr_exec);

            let attr_name = api_out_attr.rename.clone().unwrap_or_else(|| {
                // Try to infer name from expression, or generate. Very simplified.
                // Java's OutputAttribute(Variable) uses variable.getAttributeName().
                // If expression is a Variable, use its name.
                if let ApiExpression::Variable(v) = &api_out_attr.expression {
                    v.attribute_name.clone()
                } else {
                    format!("_col_{}", idx)
                }
            });
            output_attributes_for_def.push(ApiAttribute::new(attr_name, oap.get_return_type()));
            oaps.push(oap);
        }

        // Determine the ID for the stream produced by this SelectProcessor
        let output_stream_id_from_query = api_query
            .output_stream
            .get_target_id() // output_stream is not Option
            .ok_or_else(|| {
                format!(
                    "Query '{}' must have a target output stream for INSERT INTO",
                    query_name
                )
            })?;

        let mut temp_def =
            ApiStreamDefinition::new(format!("_internal_{}_select_output", query_name));
        for attr in output_attributes_for_def {
            temp_def.abstract_definition.attribute_list.push(attr);
        }
        let select_output_stream_def = Arc::new(temp_def);

        let having_executor = if let Some(expr) = &api_selector.having_expression {
            Some(parse_expression(expr, &expr_parser_context).map_err(|e| e.to_string())?)
        } else {
            None
        };

        let mut group_execs = Vec::new();
        for var in &api_selector.group_by_list {
            let expr = ApiExpression::Variable(var.clone());
            group_execs.push(parse_expression(&expr, &expr_parser_context).map_err(|e| e.to_string())?);
        }
        let group_by_key_generator = if group_execs.is_empty() {
            None
        } else {
            Some(GroupByKeyGenerator::new(group_execs))
        };

        let mut order_execs = Vec::new();
        let mut order_flags = Vec::new();
        for ob in &api_selector.order_by_list {
            let expr = ApiExpression::Variable(ob.get_variable().clone());
            order_execs.push(parse_expression(&expr, &expr_parser_context).map_err(|e| e.to_string())?);
            order_flags.push(*ob.get_order() == crate::query_api::execution::query::selection::order_by_attribute::Order::Asc);
        }
        let order_by_comparator = if order_execs.is_empty() {
            None
        } else {
            Some(OrderByEventComparator::new(order_execs, order_flags))
        };

        let select_processor = Arc::new(Mutex::new(SelectProcessor::new(
            api_selector,
            true,
            true,
            Arc::clone(siddhi_app_context),
            Arc::clone(&siddhi_query_context),
            oaps,
            select_output_stream_def,
            having_executor,
            group_by_key_generator,
            order_by_comparator,
            None,
        )));
        link_processor(select_processor.clone());

        if let Some(rate) = api_query.get_output_rate() {
            if let crate::query_api::execution::query::output::ratelimit::OutputRateVariant::Events(ev, beh) = &rate.variant {
                let limiter = Arc::new(Mutex::new(OutputRateLimiter::new(
                    None,
                    Arc::clone(siddhi_app_context),
                    Arc::clone(&siddhi_query_context),
                    ev.event_count as usize,
                    *beh,
                )));
                link_processor(limiter);
            }
        }

        // 6. Output Processor (e.g., InsertIntoStreamProcessor)
        // This needs to match on api_query.output_stream.action
        match &api_query.output_stream.action {
            crate::query_api::execution::query::output::output_stream::OutputStreamAction::InsertInto(insert_action) => {
                if let Some(target_junction) = stream_junction_map.get(&insert_action.target_id) {
                    let insert_processor = Arc::new(Mutex::new(InsertIntoStreamProcessor::new(
                        target_junction.clone(),
                        Arc::clone(siddhi_app_context),
                        Arc::clone(&siddhi_query_context),
                    )));
                    link_processor(insert_processor);
                } else if let Some(table) = siddhi_app_context.get_siddhi_context().get_table(&insert_action.target_id) {
                    let insert_processor = Arc::new(Mutex::new(
                        crate::core::query::output::InsertIntoTableProcessor::new(
                            table,
                            Arc::clone(siddhi_app_context),
                            Arc::clone(&siddhi_query_context),
                        ),
                    ));
                    link_processor(insert_processor);
                } else if let Some(agg) = aggregation_map.get(&insert_action.target_id) {
                    let insert_processor = Arc::new(Mutex::new(
                        crate::core::query::output::InsertIntoAggregationProcessor::new(
                            Arc::clone(agg),
                            Arc::clone(siddhi_app_context),
                            Arc::clone(&siddhi_query_context),
                        ),
                    ));
                    link_processor(insert_processor);
                } else {
                    return Err(format!(
                        "Output target '{}' not found for query '{}'",
                        insert_action.target_id, query_name
                    ));
                }
            }
            crate::query_api::execution::query::output::output_stream::OutputStreamAction::Update(update_action) => {
                if let Some(table) = siddhi_app_context.get_siddhi_context().get_table(&update_action.target_id) {
                    let update_processor = Arc::new(Mutex::new(
                        crate::core::query::output::UpdateTableProcessor::new(
                            table,
                            Arc::clone(siddhi_app_context),
                            Arc::clone(&siddhi_query_context),
                        ),
                    ));
                    link_processor(update_processor);
                } else {
                    return Err(format!(
                        "Update target '{}' not found for query '{}'",
                        update_action.target_id, query_name
                    ));
                }
            }
            crate::query_api::execution::query::output::output_stream::OutputStreamAction::Delete(delete_action) => {
                if let Some(table) = siddhi_app_context.get_siddhi_context().get_table(&delete_action.target_id) {
                    let delete_processor = Arc::new(Mutex::new(
                        crate::core::query::output::DeleteTableProcessor::new(
                            table,
                            Arc::clone(siddhi_app_context),
                            Arc::clone(&siddhi_query_context),
                        ),
                    ));
                    link_processor(delete_processor);
                } else {
                    return Err(format!(
                        "Delete target '{}' not found for query '{}'",
                        delete_action.target_id, query_name
                    ));
                }
            }
            _ => return Err(format!("Query '{}': Only INSERT INTO, UPDATE, DELETE outputs supported for now.", query_name)),
        }

        // 7. Create QueryRuntime
        let mut query_runtime = QueryRuntime::new_with_context(
            query_name.clone(),
            Arc::new(api_query.clone()),
            Arc::clone(&siddhi_query_context),
        );
        query_runtime.processor_chain_head = processor_chain_head;

        // 8. Register the entry processor with the input stream junction if applicable
        if let Some(head_proc_arc) = &query_runtime.processor_chain_head {
            if let Some(junction) = stream_junction_map.get(&expr_parser_context.default_source) {
                junction
                    .lock()
                    .expect("Input junction Mutex poisoned")
                    .subscribe(Arc::clone(head_proc_arc));
            }
        }

        Ok(query_runtime)
    }
}
