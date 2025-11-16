// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::core::config::eventflux_app_context::EventFluxAppContext;
use crate::core::event::event::Event;
use crate::core::event::value::AttributeValue;
use crate::core::table::{InMemoryCompiledCondition, InMemoryCompiledUpdateSet, Table};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct TableInputHandler {
    pub eventflux_app_context: Arc<EventFluxAppContext>,
    table: Arc<dyn Table>,
}

impl TableInputHandler {
    pub fn new(table: Arc<dyn Table>, eventflux_app_context: Arc<EventFluxAppContext>) -> Self {
        Self {
            eventflux_app_context,
            table,
        }
    }

    pub fn add(&self, events: Vec<Event>) {
        for event in events {
            if let Err(e) = self.table.insert(&event.data) {
                log::error!("Failed to insert event into table: {}", e);
            }
        }
    }

    pub fn update(&self, old: Vec<AttributeValue>, new: Vec<AttributeValue>) -> bool {
        let cond = InMemoryCompiledCondition { values: old };
        let us = InMemoryCompiledUpdateSet { values: new };
        match self.table.update(&cond, &us) {
            Ok(result) => result,
            Err(e) => {
                log::error!("Failed to update table: {}", e);
                false
            }
        }
    }

    pub fn delete(&self, values: Vec<AttributeValue>) -> bool {
        let cond = InMemoryCompiledCondition { values };
        match self.table.delete(&cond) {
            Ok(result) => result,
            Err(e) => {
                log::error!("Failed to delete from table: {}", e);
                false
            }
        }
    }
}
