use cortex_core::NodeId;
use cortex_signal::routing::{MultiHopMessage, MultiHopRouter, Route, RouteHop};
use cortex_signal::{Channel, Codebook, Signal, StandardSymbol};
use std::time::Instant;

fn create_test_signal() -> Signal {
    let codebook = Codebook::new();
    let symbol = StandardSymbol::Ping.to_symbol_id();
    let pattern = codebook.encode(symbol).unwrap().clone();
    Signal::new(symbol, pattern, Channel::Ble)
}

fn create_linear_topology(node_count: usize) -> Vec<(NodeId, MultiHopRouter)> {
    let nodes: Vec<NodeId> = (0..node_count).map(|_| NodeId::generate()).collect();
    let mut routers = Vec::new();

    for &node_id in nodes.iter() {
        routers.push((node_id, MultiHopRouter::new(node_id)));
    }

    routers
}

async fn setup_routes(routers: &[(NodeId, MultiHopRouter)]) {
    for i in 0..routers.len() {
        for j in (i + 1)..routers.len() {
            let source = routers[i].0;
            let dest = routers[j].0;

            let hops: Vec<RouteHop> = (i + 1..=j)
                .map(|idx| {
                    RouteHop::new(routers[idx].0, Channel::Ble).with_latency(100 * (idx - i) as u32)
                })
                .collect();

            let route = Route::new(source, dest, hops);
            routers[i].1.add_route(route).await;
        }
    }
}

async fn benchmark_routing_lookup(node_count: usize, iterations: usize) -> f64 {
    let routers = create_linear_topology(node_count);
    setup_routes(&routers).await;

    let signal = create_test_signal();
    let source = routers[0].0;
    let dest = routers[node_count - 1].0;

    let start = Instant::now();

    for _ in 0..iterations {
        let message = MultiHopMessage::new(source, dest, signal.clone());
        let _result = routers[0].1.route_message(&message).await;
    }

    let elapsed = start.elapsed();
    elapsed.as_secs_f64() / iterations as f64
}

async fn benchmark_message_forwarding(node_count: usize) -> (f64, usize) {
    let routers = create_linear_topology(node_count);
    setup_routes(&routers).await;

    let signal = create_test_signal();
    let source = routers[0].0;
    let dest = routers[node_count - 1].0;
    let mut message = MultiHopMessage::new(source, dest, signal).with_ttl(20);

    let start = Instant::now();
    let mut hops = 0;

    for i in 0..node_count - 1 {
        let current_node = routers[i].0;
        let result = routers[i].1.route_message(&message).await;
        
        match result {
            Ok(Some(_)) => {
                if message.forward(current_node).is_ok() {
                    hops += 1;
                } else {
                    break;
                }
            }
            _ => break,
        }
    }

    let elapsed = start.elapsed();
    (elapsed.as_secs_f64(), hops)
}

async fn benchmark_route_table_operations() -> f64 {
    let node_count = 100;
    let routers = create_linear_topology(node_count);

    let start = Instant::now();
    setup_routes(&routers).await;
    let elapsed = start.elapsed();

    elapsed.as_secs_f64()
}

#[tokio::main]
async fn main() {
    println!("=== Multi-Hop Routing Performance Benchmarks ===\n");

    println!("--- Benchmark 1: Routing Lookup ---");
    for node_count in [3, 5, 10, 20] {
        let avg_time = benchmark_routing_lookup(node_count, 1000).await;
        println!(
            "  {} nodes: {:.2}µs per lookup",
            node_count,
            avg_time * 1_000_000.0
        );
    }

    println!("\n--- Benchmark 2: Message Forwarding ---");
    for node_count in [3, 5, 10, 20] {
        let (total_time, hops) = benchmark_message_forwarding(node_count).await;
        println!(
            "  {} nodes: {:.2}µs total, {:.2}µs per hop ({} hops)",
            node_count,
            total_time * 1_000_000.0,
            (total_time / hops as f64) * 1_000_000.0,
            hops
        );
    }

    println!("\n--- Benchmark 3: Route Table Operations ---");
    let setup_time = benchmark_route_table_operations().await;
    println!("  100 nodes: {:.2}ms for full route setup", setup_time * 1000.0);

    println!("\n--- Memory Estimates ---");
    println!("  Route structure: ~{} bytes", std::mem::size_of::<Route>());
    println!("  RouteHop: ~{} bytes", std::mem::size_of::<RouteHop>());
    println!("  MultiHopMessage: ~{} bytes", std::mem::size_of::<MultiHopMessage>());
    println!("  Routing table (100 routes): ~{} KB", 
             (std::mem::size_of::<Route>() * 100) / 1024);

    println!("\n=== Benchmarks Complete ===");
}
