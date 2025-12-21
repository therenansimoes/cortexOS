use std::sync::Arc;
use std::time::Duration;

use cortex_core::NodeId;
use cortex_signal::{
    Channel, ConsoleEmitter, ForwardedMessage, MultiHopRouter, RouteQuality, SignalForwarder,
};
use tracing::{info, Level};
use tracing_subscriber;

fn create_node_id(n: u8) -> NodeId {
    let mut bytes = [0u8; 16];
    bytes[0] = n;
    NodeId::from_bytes(bytes)
}

async fn simulate_node(
    node_id: NodeId,
    name: &str,
    neighbors: Vec<(NodeId, Channel)>,
) -> Arc<SignalForwarder> {
    info!(node = %node_id, name = name, "Starting node");

    let router = Arc::new(MultiHopRouter::new(node_id));

    for (neighbor, channel) in neighbors {
        let path = vec![node_id, neighbor];
        let quality = RouteQuality {
            latency_us: 5000,
            reliability: 0.95,
            signal_strength: 0.9,
            hop_count: 1,
            channel: channel.clone(),
        };

        router.add_discovered_route(neighbor, path, quality).await;
        info!(
            node = %node_id,
            neighbor = %neighbor,
            channel = ?channel,
            "Added direct route to neighbor"
        );
    }

    let forwarder = Arc::new(SignalForwarder::new(node_id, router));

    let light_emitter = Arc::new(ConsoleEmitter::new(
        Channel::Light,
        format!("{}-Light", name),
    ));
    let ble_emitter = Arc::new(ConsoleEmitter::new(Channel::Ble, format!("{}-BLE", name)));

    forwarder
        .register_emitter(Channel::Light, light_emitter)
        .await;
    forwarder.register_emitter(Channel::Ble, ble_emitter).await;

    forwarder
}

async fn setup_multi_hop_chain() {
    info!("=== Setting up 4-node multi-hop chain ===");

    let node1 = create_node_id(1);
    let node2 = create_node_id(2);
    let node3 = create_node_id(3);
    let node4 = create_node_id(4);

    let forwarder1 = simulate_node(node1, "Node-1 (Source)", vec![(node2, Channel::Light)]).await;

    let forwarder2 = simulate_node(
        node2,
        "Node-2 (Relay-1)",
        vec![(node1, Channel::Light), (node3, Channel::Ble)],
    )
    .await;

    let forwarder3 = simulate_node(
        node3,
        "Node-3 (Relay-2)",
        vec![(node2, Channel::Ble), (node4, Channel::Light)],
    )
    .await;

    let forwarder4 =
        simulate_node(node4, "Node-4 (Destination)", vec![(node3, Channel::Light)]).await;

    info!("\n=== Testing multi-hop message forwarding ===\n");

    let message = ForwardedMessage::new(
        node1,
        node4,
        b"Hello from Node 1 to Node 4 via multi-hop!".to_vec(),
        5,
    );

    info!(
        source = %message.source,
        destination = %message.destination,
        payload_size = message.payload.len(),
        max_hops = message.max_hops,
        "Initiating multi-hop message transmission"
    );

    info!("\nHop 1: Node-1 → Node-2 (via Light)");
    let result = forwarder1
        .send_via_signal(node2, message.payload.clone(), Some(Channel::Light))
        .await;
    match result {
        Ok(_) => info!("  ✓ Signal sent successfully"),
        Err(e) => info!("  ✗ Error: {:?}", e),
    }

    info!("\nHop 2: Node-2 → Node-3 (via BLE)");
    let result = forwarder2
        .send_via_signal(node3, message.payload.clone(), Some(Channel::Ble))
        .await;
    match result {
        Ok(_) => info!("  ✓ Signal sent successfully"),
        Err(e) => info!("  ✗ Error: {:?}", e),
    }

    info!("\nHop 3: Node-3 → Node-4 (via Light)");
    let result = forwarder3
        .send_via_signal(node4, message.payload.clone(), Some(Channel::Light))
        .await;
    match result {
        Ok(_) => info!("  ✓ Signal sent successfully"),
        Err(e) => info!("  ✗ Error: {:?}", e),
    }

    info!("\n=== Message delivery complete ===\n");
}

async fn simulate_swarm_network() {
    info!("\n=== Simulating swarm network with mesh topology ===");

    let node_a = create_node_id(10);
    let node_b = create_node_id(11);
    let node_c = create_node_id(12);
    let node_d = create_node_id(13);
    let node_e = create_node_id(14);

    let forwarder_a = simulate_node(
        node_a,
        "Node-A",
        vec![(node_b, Channel::Ble), (node_c, Channel::Light)],
    )
    .await;

    let forwarder_b = simulate_node(
        node_b,
        "Node-B",
        vec![
            (node_a, Channel::Ble),
            (node_c, Channel::Ble),
            (node_d, Channel::Light),
        ],
    )
    .await;

    let forwarder_c = simulate_node(
        node_c,
        "Node-C",
        vec![
            (node_a, Channel::Light),
            (node_b, Channel::Ble),
            (node_e, Channel::Ble),
        ],
    )
    .await;

    let _forwarder_d = simulate_node(
        node_d,
        "Node-D",
        vec![(node_b, Channel::Light), (node_e, Channel::Light)],
    )
    .await;

    let _forwarder_e = simulate_node(
        node_e,
        "Node-E",
        vec![(node_c, Channel::Ble), (node_d, Channel::Light)],
    )
    .await;

    info!("\n=== Broadcasting from Node-A to all neighbors ===\n");

    let _ = forwarder_a
        .announce_route_discovery(node_e, Channel::Ble)
        .await;

    tokio::time::sleep(Duration::from_millis(100)).await;

    info!("\n=== Testing alternative routes ===");

    info!("\nRoute 1: A → B → D → E");
    let _ = forwarder_a
        .send_via_signal(node_b, b"via B".to_vec(), Some(Channel::Ble))
        .await;
    let _ = forwarder_b
        .send_via_signal(node_d, b"via D".to_vec(), Some(Channel::Light))
        .await;

    info!("\nRoute 2: A → C → E");
    let _ = forwarder_a
        .send_via_signal(node_c, b"via C".to_vec(), Some(Channel::Light))
        .await;
    let _ = forwarder_c
        .send_via_signal(node_e, b"direct".to_vec(), Some(Channel::Ble))
        .await;

    info!("\n=== Swarm network simulation complete ===\n");
}

async fn demonstrate_route_quality() {
    info!("\n=== Demonstrating route quality metrics ===");

    let node1 = create_node_id(20);
    let node2 = create_node_id(21);
    let node3 = create_node_id(22);

    let router = Arc::new(MultiHopRouter::new(node1));

    let high_quality = RouteQuality {
        latency_us: 1000,
        reliability: 0.99,
        signal_strength: 0.95,
        hop_count: 1,
        channel: Channel::Ble,
    };

    let low_quality = RouteQuality {
        latency_us: 50_000,
        reliability: 0.7,
        signal_strength: 0.6,
        hop_count: 3,
        channel: Channel::Light,
    };

    router
        .add_discovered_route(node2, vec![node1, node2], high_quality.clone())
        .await;
    router
        .add_discovered_route(node2, vec![node1, node3, node2], low_quality.clone())
        .await;

    info!(
        high_quality_score = high_quality.score(),
        low_quality_score = low_quality.score(),
        "Quality scores computed"
    );

    match router.find_route(&node2).await {
        Ok(route) => {
            info!(
                selected_route_hops = route.hop_count(),
                route_score = route.quality.score(),
                "Best route selected based on quality"
            );
        }
        Err(e) => info!("Route lookup failed: {:?}", e),
    }

    info!("\n=== Route quality demonstration complete ===\n");
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    info!("╔══════════════════════════════════════════════════════════╗");
    info!("║     CortexOS Multi-Hop Communication Demonstration      ║");
    info!("╚══════════════════════════════════════════════════════════╝\n");

    setup_multi_hop_chain().await;

    tokio::time::sleep(Duration::from_millis(500)).await;

    simulate_swarm_network().await;

    tokio::time::sleep(Duration::from_millis(500)).await;

    demonstrate_route_quality().await;

    info!("\n╔══════════════════════════════════════════════════════════╗");
    info!("║          Multi-Hop Demo Complete                        ║");
    info!("╚══════════════════════════════════════════════════════════╝");
}
