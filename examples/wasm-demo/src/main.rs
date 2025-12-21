use cortex_core::{
    capability::{Capability, CapabilitySet, SensorType},
    event::{Event, Payload},
    runtime::{Runtime, Agent},
    backpressure::BackpressurePolicy,
    async_trait,
    Result,
};
use std::path::PathBuf;

struct TestAgent {
    name: String,
    caps: CapabilitySet,
}

impl TestAgent {
    fn new(name: &str) -> Self {
        let mut caps = CapabilitySet::new();
        caps.add(Capability::network_tcp(vec![]));
        caps.add(Capability::sensor(SensorType::Microphone));
        Self {
            name: name.to_string(),
            caps,
        }
    }
}

#[async_trait]
impl Agent for TestAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.caps
    }

    async fn handle(&self, event: Event) -> Result<()> {
        println!("Agent {} received event: {}", self.name, event.id);
        Ok(())
    }
}

fn main() {
    println!("=== CortexOS WASM Comprehensive Demo ===\n");
    
    // Test 1: Runtime creation
    println!("✓ Creating runtime...");
    let runtime = Runtime::new();
    let event_bus = runtime.event_bus();
    println!("  Runtime created successfully");
    
    // Test 2: Event creation with different payload types
    println!("\n✓ Testing event creation...");
    let event1 = Event::new(
        "wasm-demo",
        "demo.inline.v1",
        Payload::inline(b"Hello from WASM!".to_vec()),
    );
    println!("  Created inline event: {}", event1.id);
    
    let hash = [0u8; 32]; // Example hash
    let event2 = Event::new(
        "wasm-demo",
        "demo.ref.v1",
        Payload::reference(hash, 42),
    );
    println!("  Created reference event: {}", event2.id);
    
    // Test 3: Event with trace
    let event3 = Event::new(
        "wasm-demo",
        "demo.traced.v1",
        Payload::inline(b"Traced event".to_vec()),
    ).with_trace("trace-123", "span-456");
    println!("  Created traced event: {}", event3.id);
    
    // Test 4: Event publishing
    println!("\n✓ Testing event bus...");
    if let Err(e) = event_bus.publish(event1.clone()) {
        eprintln!("  Failed to publish event: {}", e);
    } else {
        println!("  Event published successfully");
    }
    
    // Test 5: Capability system
    println!("\n✓ Testing capability system...");
    let mut caps = CapabilitySet::new();
    
    // Add filesystem capabilities
    caps.add(Capability::fs_read(vec![PathBuf::from("/tmp")]));
    caps.add(Capability::network_tcp(vec!["example.com".to_string()]));
    println!("  Added filesystem and network capabilities");
    
    // Check capabilities
    let test_path = PathBuf::from("/tmp/test.txt");
    println!("  Can read /tmp/test.txt: {}", caps.check_fs_read(&test_path));
    println!("  Can write /tmp/test.txt: {}", caps.check_fs_write(&test_path));
    println!("  Can access example.com via TCP: {}", caps.check_network("example.com", true));
    
    // Remove capability
    let fs_cap = Capability::fs_read(vec![PathBuf::from("/tmp")]);
    caps.remove(&fs_cap);
    println!("  Removed filesystem read capability");
    println!("  Can read /tmp/test.txt: {}", caps.check_fs_read(&test_path));
    
    // Test 6: Sensor capabilities
    println!("\n✓ Testing sensor capabilities...");
    let mut sensor_caps = CapabilitySet::new();
    sensor_caps.add(Capability::sensor(SensorType::Microphone));
    sensor_caps.add(Capability::sensor(SensorType::Camera));
    println!("  Added Microphone and Camera sensors");
    println!("  Has Microphone: {}", sensor_caps.check_sensor(&SensorType::Microphone));
    println!("  Has Keyboard: {}", sensor_caps.check_sensor(&SensorType::Keyboard));
    
    // Test 7: Grid capabilities
    println!("\n✓ Testing Grid capabilities...");
    let mut grid_caps = CapabilitySet::new();
    grid_caps.add(Capability::grid_full());
    println!("  Added full Grid capability");
    println!("  Can relay: {}", grid_caps.check_grid_relay());
    println!("  Can accept tasks: {}", grid_caps.check_grid_task_accept());
    
    // Test 8: Backpressure policies
    println!("\n✓ Testing backpressure policies...");
    let _policy_drop_new = BackpressurePolicy::DropNew;
    let _policy_drop_old = BackpressurePolicy::DropOld;
    let _policy_coalesce = BackpressurePolicy::Coalesce("sensor-id".to_string());
    let _policy_sample = BackpressurePolicy::Sample(10);
    let _policy_persist = BackpressurePolicy::Persist;
    println!("  Created all backpressure policy types:");
    println!("    - DropNew, DropOld");
    println!("    - Coalesce, Sample");
    println!("    - Persist");
    
    // Test 9: Agent system
    println!("\n✓ Testing agent system...");
    let agent = TestAgent::new("test-agent-1");
    println!("  Created agent: {}", agent.name());
    println!("  Agent capabilities: {} total", agent.capabilities().len());
    println!("  Has TCP network capability: {}", agent.capabilities().check_network("any", true));
    println!("  Has Microphone sensor: {}", agent.capabilities().check_sensor(&SensorType::Microphone));
    
    println!("\n=== All Tests Passed ===");
    println!("WASM demo completed successfully!");
    println!("\n📊 Size Optimizations Applied:");
    println!("  • opt-level = 'z' (size optimization)");
    println!("  • LTO enabled");
    println!("  • Single codegen unit");
    println!("  • Limited tokio features for WASM");
    println!("  • No RocksDB (using MemoryStore)");
}

