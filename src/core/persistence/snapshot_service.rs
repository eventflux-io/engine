// SPDX-License-Identifier: MIT OR Apache-2.0

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::core::persistence::StateHolder;
use crate::core::util::{from_bytes, to_bytes};

use super::persistence_store::PersistenceStore;

/// Basic snapshot service keeping arbitrary state bytes.
#[derive(Default)]
pub struct SnapshotService {
    state: Mutex<Vec<u8>>, // serialized runtime state
    pub persistence_store: Option<Arc<dyn PersistenceStore>>,
    pub eventflux_app_id: String,
    state_holders: Mutex<HashMap<String, Arc<Mutex<dyn StateHolder>>>>,
}

#[derive(Serialize, Deserialize, Default)]
struct SnapshotData {
    main: Vec<u8>,
    holders: HashMap<String, crate::core::persistence::StateSnapshot>,
}

impl std::fmt::Debug for SnapshotService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SnapshotService")
            .field("eventflux_app_id", &self.eventflux_app_id)
            .finish()
    }
}

impl SnapshotService {
    pub fn new(eventflux_app_id: String) -> Self {
        Self {
            state: Mutex::new(Vec::new()),
            persistence_store: None,
            eventflux_app_id,
            state_holders: Mutex::new(HashMap::new()),
        }
    }

    /// Replace the current internal state.
    pub fn set_state(&self, data: Vec<u8>) {
        *self.state.lock().unwrap() = data;
    }

    /// Register a state holder to be included in snapshots.
    pub fn register_state_holder(&self, id: String, holder: Arc<Mutex<dyn StateHolder>>) {
        self.state_holders.lock().unwrap().insert(id, holder);
    }

    /// Retrieve a copy of the internal state.
    pub fn snapshot(&self) -> Vec<u8> {
        self.state.lock().unwrap().clone()
    }

    /// Persist the current state via the configured store.
    pub fn persist(&self) -> Result<String, String> {
        let mut holders = HashMap::new();
        let hints = crate::core::persistence::SerializationHints::default();
        for (id, holder) in self.state_holders.lock().unwrap().iter() {
            println!("Persisting state for component: {}", id);
            match holder.lock().unwrap().serialize_state(&hints) {
                Ok(snapshot) => {
                    holders.insert(id.clone(), snapshot);
                    println!("Successfully persisted state for: {}", id);
                }
                Err(e) => {
                    eprintln!("Failed to serialize state for {id}: {e:?}");
                    continue;
                }
            }
        }
        let snapshot = SnapshotData {
            main: self.snapshot(),
            holders,
        };
        let data = to_bytes(&snapshot).map_err(|e| e.to_string())?;
        let store = self
            .persistence_store
            .as_ref()
            .ok_or("No persistence store")?;
        let revision = Utc::now().timestamp_millis().to_string();
        store.save(&self.eventflux_app_id, &revision, &data);
        Ok(revision)
    }

    /// Load the given revision from the store and set as current state.
    pub fn restore_revision(&self, revision: &str) -> Result<(), String> {
        let store = self
            .persistence_store
            .as_ref()
            .ok_or("No persistence store")?;
        if let Some(data) = store.load(&self.eventflux_app_id, revision) {
            let snap: SnapshotData = from_bytes(&data).map_err(|e| e.to_string())?;
            self.set_state(snap.main);
            for (id, snapshot) in snap.holders {
                println!("Restoring state for component: {}", id);
                if let Some(holder) = self.state_holders.lock().unwrap().get(&id) {
                    // Use the full snapshot with all metadata (compression, checksum, version, etc.)
                    match holder.lock().unwrap().deserialize_state(&snapshot) {
                        Ok(_) => println!("Successfully restored state for: {}", id),
                        Err(e) => {
                            eprintln!("Failed to restore state for {id}: {e:?}");
                            eprintln!("Component ID: {}, Error details: {}", id, e);
                        }
                    }
                } else {
                    println!("No state holder found for component: {}", id);
                }
            }
            Ok(())
        } else {
            Err("Revision not found".into())
        }
    }
}
