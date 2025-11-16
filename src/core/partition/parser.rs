// SPDX-License-Identifier: MIT OR Apache-2.0

use std::sync::Arc;

use crate::core::config::eventflux_app_context::EventFluxAppContext;
use crate::core::eventflux_app_runtime_builder::EventFluxAppRuntimeBuilder;
use crate::core::partition::PartitionRuntime;
use crate::core::util::parser::QueryParser;
use crate::query_api::execution::partition::Partition as ApiPartition;

pub struct PartitionParser;

impl PartitionParser {
    pub fn parse(
        builder: &mut EventFluxAppRuntimeBuilder,
        partition: &ApiPartition,
        eventflux_app_context: &Arc<EventFluxAppContext>,
        partition_index: usize,
    ) -> Result<PartitionRuntime, String> {
        let mut partition_runtime = PartitionRuntime::new();

        // Determine unique partition ID from @info(name='...') annotation or generate from index
        let partition_id = partition
            .annotations
            .iter()
            .find(|ann| ann.name == "info")
            .and_then(|ann| ann.elements.iter().find(|el| el.key == "name"))
            .map(|el| el.value.clone())
            .unwrap_or_else(|| format!("partition_{}", partition_index));

        // ensure a named executor for partition queries exists
        eventflux_app_context
            .get_eventflux_context()
            .executor_services
            .get_or_create_from_env("partition", 2);

        for (query_index, query) in partition.query_list.iter().enumerate() {
            let qr = QueryParser::parse_query(
                query,
                eventflux_app_context,
                &builder.stream_junction_map,
                &builder.table_definition_map,
                &builder.aggregation_map,
                Some(partition_id.clone()),
                query_index,
            )?;
            let qr_arc = Arc::new(qr);
            builder.add_query_runtime(Arc::clone(&qr_arc));
            partition_runtime.add_query_runtime(qr_arc);
        }
        Ok(partition_runtime)
    }
}
