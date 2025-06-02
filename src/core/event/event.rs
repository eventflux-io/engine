// siddhi_rust/src/core/event/event.rs
// Corresponds to io.siddhi.core.event.Event
use super::value::AttributeValue;
use std::sync::atomic::{AtomicU64, Ordering};

// Global atomic counter for generating unique event IDs.
// Siddhi Event.java does not have an explicit ID field. This is an addition for Rust if needed,
// or can be removed if events are identified by other means (e.g. object identity in a Vec).
// The prompt includes `id: u64`.
static NEXT_EVENT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Event {
    pub id: u64, // Unique ID, added as per prompt
    pub timestamp: i64, // Java default -1
    pub data: Vec<AttributeValue>, // Java Object[] data
    pub is_expired: bool, // Java default false
    // Java Event also has 'expiryTime', which is not standard in the base Event but used in ComplexEvent/StreamEvent.
    // For now, aligning with the basic Event.java fields + ID.
}

impl Event {
    // Constructor matching Event(long timestamp, Object[] data)
    // Data is passed directly.
    pub fnnew_with_data(timestamp: i64, data: Vec<AttributeValue>) -> Self {
        Event {
            id: NEXT_EVENT_ID.fetch_add(1, Ordering::Relaxed), // Relaxed ordering sufficient for unique ID
            timestamp,
            data,
            is_expired: false,
        }
    }

    // Constructor matching Event(int dataSize), initializes with nulls/defaults
    pub fnnew_with_size(timestamp: i64, data_len: usize) -> Self {
        Event {
            id: NEXT_EVENT_ID.fetch_add(1, Ordering::Relaxed),
            timestamp,
            data: vec![AttributeValue::default(); data_len], // Fill with AttributeValue::Null
            is_expired: false,
        }
    }

    // Corresponds to no-arg Event() which makes data = new Object[0]
    // And Event(long timestamp, Object[] data) where data could be empty.
    // Default::default() already provides an empty data Vec.
    // This new() can be used for default timestamp (-1).
    pub fn new_empty(timestamp: i64) -> Self {
        Event {
            id: NEXT_EVENT_ID.fetch_add(1, Ordering::Relaxed),
            timestamp,
            data: Vec::new(),
            is_expired: false,
        }
    }

    // --- Methods from Java Event.java ---
    pub fn get_timestamp(&self) -> i64 {
        self.timestamp
    }

    pub fn set_timestamp(&mut self, timestamp: i64) {
        self.timestamp = timestamp;
    }

    pub fn get_data(&self) -> &Vec<AttributeValue> {
        &self.data
    }

    pub fn get_data_mut(&mut self) -> &mut Vec<AttributeValue> {
        &mut self.data
    }

    pub fn set_data(&mut self, data: Vec<AttributeValue>) {
        self.data = data;
    }

    pub fn get_data_at_idx(&self, i: usize) -> Option<&AttributeValue> {
        self.data.get(i)
    }

    pub fn set_data_at_idx(&mut self, i: usize, value: AttributeValue) -> Result<(), String> {
        if i < self.data.len() {
            self.data[i] = value;
            Ok(())
        } else {
            Err(format!("Index {} out of bounds for event data with len {}", i, self.data.len()))
        }
    }

    pub fn is_expired(&self) -> bool {
        self.is_expired
    }

    pub fn set_is_expired(&mut self, is_expired: bool) {
        self.is_expired = is_expired;
    }

    // copy_from(Event event)
    // This needs to handle id carefully. Typically copy methods don't create new IDs.
    // For now, it copies data but keeps its own ID.
    pub fn copy_from(&mut self, other_event: &Event) {
        self.timestamp = other_event.timestamp;
        self.is_expired = other_event.is_expired;
        // Deep copy data if AttributeValue variants require it (String, Object already are).
        // Vec clone will do the right thing for AttributeValue::String, etc.
        // For Box<dyn Any> in Object, clone will copy the Box (pointer), not the data.
        // This is usually fine unless true deep object cloning is needed.
        self.data = other_event.data.clone();
    }

    // copyFrom(ComplexEvent complexEvent) - This will be tricky as ComplexEvent is a trait.
    // It implies ComplexEvent needs methods to expose its data in a compatible format.
    // pub fn copy_from_complex(&mut self, complex_event: &dyn ComplexEvent) {
    //     self.timestamp = complex_event.get_timestamp();
    //     // Assuming complex_event.get_output_data() returns something convertible to Vec<AttributeValue>
    //     // and complex_event.get_type() returns something mappable to is_expired.
    //     // This requires ComplexEvent trait to be more defined.
    //     unimplemented!("copy_from_complex");
    // }
}
