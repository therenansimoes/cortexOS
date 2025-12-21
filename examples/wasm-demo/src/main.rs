use cortex_core::{
    event::{Event, Payload},
    runtime::Runtime,
};

fn main() {
    println!("CortexOS WASM Demo");
    
    // Create runtime
    let runtime = Runtime::new();
    let event_bus = runtime.event_bus();
    
    // Create a simple event
    let event = Event::new(
        "wasm-demo",
        "demo.hello.v1",
        Payload::inline(b"Hello from WASM!".to_vec()),
    );
    
    println!("Created event with ID: {}", event.id);
    
    // Publish event
    if let Err(e) = event_bus.publish(event) {
        eprintln!("Failed to publish event: {}", e);
    } else {
        println!("Event published successfully!");
    }
    
    println!("WASM demo completed successfully");
}
