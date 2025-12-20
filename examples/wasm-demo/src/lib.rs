//! WASM Demo - CortexOS Core Running in WebAssembly
//!
//! This example demonstrates that the CortexOS core can be compiled to WASM
//! and run in a browser or WASI runtime.

use cortex_core::event::{Event, EventMetrics, Payload};
use cortex_core::backpressure::{BackpressurePolicy, PolicyQueue, Keyed};

#[derive(Clone, Debug)]
struct SensorReading {
    sensor_id: String,
    value: f64,
}

impl Keyed for SensorReading {
    fn key(&self) -> Option<&str> {
        Some(&self.sensor_id)
    }
}

/// Initialize the WASM module
#[no_mangle]
pub extern "C" fn init() -> i32 {
    0
}

/// Create and validate an event
#[no_mangle]
pub extern "C" fn create_event() -> i32 {
    let event = Event::new("wasm-demo", "sensor.temp.v1", Payload::inline(vec![1, 2, 3]));
    
    match event.validate() {
        Ok(_) => 1,
        Err(_) => 0,
    }
}

/// Test backpressure policy
#[no_mangle]
pub extern "C" fn test_backpressure() -> i32 {
    let queue: PolicyQueue<SensorReading> = PolicyQueue::new(BackpressurePolicy::DropOld, 10);
    
    for i in 0..20 {
        let reading = SensorReading {
            sensor_id: format!("sensor{}", i % 5),
            value: i as f64,
        };
        let _ = queue.push(reading);
    }
    
    queue.len() as i32
}

/// Get event metrics
#[no_mangle]
pub extern "C" fn get_metrics() -> i32 {
    let metrics = EventMetrics::snapshot();
    metrics.events_created as i32
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    #[no_mangle]
    pub extern "C" fn main() {
        super::init();
    }
}
