use cortex_core::NodeId;
use cortex_signal::{
    Channel, Codebook, ConsoleEmitter, Emitter, MultiHopMessage, MultiHopRouter, Pulse, Route,
    RouteHop, Signal, SignalPattern, StandardSymbol,
};
use std::sync::Arc;
use tracing::{info, Level};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("=== Multi-Hop Communication Demo ===");
    info!("Demonstrating signal routing across multiple nodes in a mesh network");

    // Create a 4-node network topology:
    // NodeA -> NodeB -> NodeC -> NodeD
    // NodeA wants to send a message to NodeD through B and C

    let node_a = NodeId::generate();
    let node_b = NodeId::generate();
    let node_c = NodeId::generate();
    let node_d = NodeId::generate();

    info!("\n--- Network Topology ---");
    info!("Node A: {:?}", node_a);
    info!("Node B: {:?}", node_b);
    info!("Node C: {:?}", node_c);
    info!("Node D: {:?}", node_d);
    info!("Path: A -> B -> C -> D");

    // Create routers for each node
    let router_a = Arc::new(MultiHopRouter::new(node_a));
    let router_b = Arc::new(MultiHopRouter::new(node_b));
    let router_c = Arc::new(MultiHopRouter::new(node_c));
    let router_d = Arc::new(MultiHopRouter::new(node_d));

    // Setup routes in each router
    // Route from A to D through B and C
    info!("\n--- Setting Up Routes ---");
    
    let route_a_to_d = Route::new(
        node_a,
        node_d,
        vec![
            RouteHop::new(node_b, Channel::Ble).with_latency(1000),
            RouteHop::new(node_c, Channel::Light).with_latency(1500),
            RouteHop::new(node_d, Channel::Audio).with_latency(2000),
        ],
    );
    router_a.add_route(route_a_to_d.clone()).await;
    info!("Added route at Node A: A -> B -> C -> D");

    // Route from B to D through C
    let route_b_to_d = Route::new(
        node_b,
        node_d,
        vec![
            RouteHop::new(node_c, Channel::Light).with_latency(1500),
            RouteHop::new(node_d, Channel::Audio).with_latency(2000),
        ],
    );
    router_b.add_route(route_b_to_d).await;
    info!("Added route at Node B: B -> C -> D");

    // Route from C to D
    let route_c_to_d = Route::new(
        node_c,
        node_d,
        vec![RouteHop::new(node_d, Channel::Audio).with_latency(2000)],
    );
    router_c.add_route(route_c_to_d).await;
    info!("Added route at Node C: C -> D");

    // Create a test signal to send
    let codebook = Codebook::new();
    let symbol = StandardSymbol::TaskRequest.to_symbol_id();
    let pattern = codebook.encode(symbol).unwrap().clone();
    let signal = Signal::new(symbol, pattern, Channel::Ble);

    info!("\n--- Creating Message ---");
    info!("Creating message with symbol: {:?}", StandardSymbol::TaskRequest);
    
    let mut message = MultiHopMessage::new(node_a, node_d, signal);
    info!("Initial TTL: {}", message.ttl);
    info!("Initial hop count: {}", message.hop_count);

    // Simulate routing the message through the network
    info!("\n--- Simulating Multi-Hop Routing ---");

    // At Node A
    info!("\n[Node A] Processing message");
    let next_hop_a = router_a.route_message(&message).await.unwrap();
    if let Some(hop) = next_hop_a {
        message.forward(node_a).unwrap();
        info!("[Node A] Forwarding to Node B via {:?}", hop.channel);
        info!("[Node A] Updated hop count: {}, TTL: {}", message.hop_count, message.ttl);
    }

    // At Node B
    info!("\n[Node B] Receiving message");
    let next_hop_b = router_b.route_message(&message).await.unwrap();
    if let Some(hop) = next_hop_b {
        message.forward(node_b).unwrap();
        info!("[Node B] Forwarding to Node C via {:?}", hop.channel);
        info!("[Node B] Updated hop count: {}, TTL: {}", message.hop_count, message.ttl);
    }

    // At Node C
    info!("\n[Node C] Receiving message");
    let next_hop_c = router_c.route_message(&message).await.unwrap();
    if let Some(hop) = next_hop_c {
        message.forward(node_c).unwrap();
        info!("[Node C] Forwarding to Node D via {:?}", hop.channel);
        info!("[Node C] Updated hop count: {}, TTL: {}", message.hop_count, message.ttl);
    }

    // At Node D (destination)
    info!("\n[Node D] Receiving message");
    let next_hop_d = router_d.route_message(&message).await.ok();
    if next_hop_d.is_none() {
        info!("[Node D] Message reached destination!");
        info!("[Node D] Total hops: {}", message.hop_count);
        info!("[Node D] Route taken: {:?}", message.route_record);
    }

    // Demonstrate emitting signals on different channels
    info!("\n--- Physical Signal Emission (Simulated) ---");
    let ble_emitter = ConsoleEmitter::new(Channel::Ble, "Node-B");
    let light_emitter = ConsoleEmitter::new(Channel::Light, "Node-C");
    let audio_emitter = ConsoleEmitter::new(Channel::Audio, "Node-D");

    let test_pattern = SignalPattern::new(vec![
        Pulse::on(1000),
        Pulse::off(500),
        Pulse::on(1000),
    ]);
    
    let test_signal = Signal::new(symbol, test_pattern, Channel::Ble);

    info!("\nEmitting on BLE (Node B):");
    ble_emitter.emit_signal(&test_signal, &codebook).await.unwrap();

    let light_signal = Signal::new(symbol, test_signal.pattern.clone(), Channel::Light);
    info!("\nEmitting on Light (Node C):");
    light_emitter.emit_signal(&light_signal, &codebook).await.unwrap();

    let audio_signal = Signal::new(symbol, test_signal.pattern.clone(), Channel::Audio);
    info!("\nEmitting on Audio (Node D):");
    audio_emitter.emit_signal(&audio_signal, &codebook).await.unwrap();

    // Display route statistics
    info!("\n--- Route Statistics ---");
    info!("Total latency for A->D route: {:?}µs", route_a_to_d.total_latency_us());
    info!("Route quality score: {:.2}", route_a_to_d.quality_score());
    info!("Route hop count: {}", route_a_to_d.hop_count());

    // Demonstrate route discovery
    info!("\n--- Route Discovery ---");
    let discovery_request = router_a.discover_route(node_d).await;
    info!("Started route discovery from {:?} to {:?}", discovery_request.source, discovery_request.destination);
    info!("Discovery request ID: {:?}", discovery_request.id);

    // Show router states
    info!("\n--- Router States ---");
    info!("Router A: {} routes", router_a.route_count().await);
    info!("Router B: {} routes", router_b.route_count().await);
    info!("Router C: {} routes", router_c.route_count().await);
    info!("Router D: {} routes", router_d.route_count().await);

    info!("\n=== Demo Complete ===");
}
