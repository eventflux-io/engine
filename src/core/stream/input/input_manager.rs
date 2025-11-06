// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::input_distributor::InputDistributor;
use super::input_entry_valve::InputEntryValve;
use super::input_handler::{InputHandler, InputProcessor};
use super::table_input_handler::TableInputHandler;
use crate::core::config::eventflux_app_context::EventFluxAppContext;
use crate::core::stream::stream_junction::StreamJunction;
use crate::core::util::thread_barrier::ThreadBarrier;

#[derive(Debug)]
pub struct InputManager {
    eventflux_app_context: Arc<EventFluxAppContext>,
    input_handlers: Mutex<HashMap<String, Arc<Mutex<InputHandler>>>>,
    table_input_handlers: Mutex<HashMap<String, TableInputHandler>>,
    stream_junction_map: HashMap<String, Arc<Mutex<StreamJunction>>>,
    input_distributor: Arc<Mutex<InputDistributor>>,
    input_entry_valve: Arc<Mutex<dyn InputProcessor>>,
    is_connected: Mutex<bool>,
}

impl InputManager {
    pub fn new(
        eventflux_app_context: Arc<EventFluxAppContext>,
        stream_junction_map: HashMap<String, Arc<Mutex<StreamJunction>>>,
    ) -> Self {
        let distributor: Arc<Mutex<InputDistributor>> =
            Arc::new(Mutex::new(InputDistributor::default()));
        let distributor_for_valve: Arc<Mutex<dyn InputProcessor>> = distributor.clone();
        let barrier = eventflux_app_context
            .get_thread_barrier()
            .unwrap_or_else(|| Arc::new(ThreadBarrier::new()));
        let entry_valve: Arc<Mutex<dyn InputProcessor>> = Arc::new(Mutex::new(
            InputEntryValve::new(barrier, distributor_for_valve),
        ));
        Self {
            eventflux_app_context,
            input_handlers: Mutex::new(HashMap::new()),
            table_input_handlers: Mutex::new(HashMap::new()),
            stream_junction_map,
            input_distributor: distributor,
            input_entry_valve: entry_valve,
            is_connected: Mutex::new(false),
        }
    }

    pub fn construct_input_handler(
        &self,
        stream_id: &str,
    ) -> Result<Arc<Mutex<InputHandler>>, String> {
        let junction = self
            .stream_junction_map
            .get(stream_id)
            .ok_or_else(|| format!("StreamJunction '{stream_id}' not found"))?
            .clone();
        let publisher = StreamJunction::construct_publisher(junction);
        self.input_distributor
            .lock()
            .map_err(|_| "distributor mutex".to_string())?
            .add_input_processor(Arc::new(Mutex::new(publisher.clone())));
        let handler = Arc::new(Mutex::new(InputHandler::new(
            stream_id.to_string(),
            self.input_handlers.lock().unwrap().len(),
            self.input_entry_valve.clone(),
            Arc::clone(&self.eventflux_app_context),
        )));
        self.input_handlers
            .lock()
            .unwrap()
            .insert(stream_id.to_string(), Arc::clone(&handler));
        Ok(handler)
    }

    pub fn get_input_handler(&self, stream_id: &str) -> Option<Arc<Mutex<InputHandler>>> {
        if let Some(h) = self.input_handlers.lock().unwrap().get(stream_id) {
            return Some(h.clone());
        }
        self.construct_input_handler(stream_id).ok()
    }

    pub fn get_table_input_handler(&self, table_id: &str) -> Option<TableInputHandler> {
        if let Some(h) = self.table_input_handlers.lock().unwrap().get(table_id) {
            return Some(h.clone());
        }
        self.construct_table_input_handler(table_id).ok()
    }

    pub fn construct_table_input_handler(
        &self,
        table_id: &str,
    ) -> Result<TableInputHandler, String> {
        let table = self
            .eventflux_app_context
            .get_eventflux_context()
            .get_table(table_id)
            .ok_or_else(|| format!("Table '{table_id}' not found"))?;
        let handler = TableInputHandler::new(table, Arc::clone(&self.eventflux_app_context));
        self.table_input_handlers
            .lock()
            .unwrap()
            .insert(table_id.to_string(), handler.clone());
        Ok(handler)
    }

    pub fn connect(&self) {
        for handler in self.input_handlers.lock().unwrap().values() {
            if let Ok(mut h) = handler.lock() {
                h.connect();
            }
        }
        *self.is_connected.lock().unwrap() = true;
    }

    pub fn disconnect(&self) {
        for handler in self.input_handlers.lock().unwrap().values() {
            if let Ok(mut h) = handler.lock() {
                h.disconnect();
            }
        }
        *self.is_connected.lock().unwrap() = false;
    }
}
