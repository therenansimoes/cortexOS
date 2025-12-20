# Multi-Hop Communication Demo

This example demonstrates the multi-hop communication capabilities of CortexOS, showing how signals can be routed across multiple physical devices in a mesh network.

## Overview

Multi-hop communication enables CortexOS nodes to send signals to distant nodes by relaying them through intermediate nodes. This is essential for:

- **Extended range**: Reach nodes beyond direct communication distance
- **Mesh networking**: Create resilient, self-organizing networks
- **Swarm coordination**: Enable cooperation among distributed robots/devices
- **Censorship resistance**: Route around blocked or unavailable paths

## Network Topology

The demo creates a 4-node linear topology:

```
Node A ──(BLE)──> Node B ──(Light)──> Node C ──(Audio)──> Node D
```

- **Node A**: Source - wants to send a message to Node D
- **Node B**: First relay - forwards using BLE signal
- **Node C**: Second relay - forwards using light signal
- **Node D**: Destination - receives the message

## Key Features Demonstrated

### 1. Route Management
- **Route creation**: Static routes defined for each node
- **Route storage**: Routing tables maintain multiple paths
- **Route selection**: Best route chosen based on quality score

### 2. Message Forwarding
- **TTL (Time To Live)**: Prevents infinite loops
- **Hop counting**: Tracks message progress
- **Route recording**: Maintains path history

### 3. Multi-Channel Support
- **BLE**: Short-range wireless (Node B)
- **Light**: Optical signals (Node C)
- **Audio**: Sound-based (Node D)
- **Automatic selection**: Best channel per hop

### 4. Route Quality
- **Success rate**: Tracks delivery statistics
- **Hop penalty**: Prefers shorter paths
- **Age decay**: Older routes scored lower
- **Latency tracking**: Per-hop timing information

## Running the Demo

```bash
cargo run -p multi-hop-demo
```

## Output Explanation

The demo shows:

1. **Network Setup**: Node IDs and topology
2. **Route Configuration**: Installed routes at each node
3. **Message Creation**: Signal details and initial state
4. **Hop-by-Hop Routing**: 
   - Message processing at each node
   - Next hop selection
   - TTL and hop count updates
5. **Physical Emission**: Simulated signal output on each channel
6. **Statistics**: Route metrics and quality scores

## Code Structure

### Route Definition
```rust
let route = Route::new(
    source_node,
    destination_node,
    vec![
        RouteHop::new(next_node_1, Channel::Ble).with_latency(1000),
        RouteHop::new(next_node_2, Channel::Light).with_latency(1500),
    ],
);
```

### Message Forwarding
```rust
let mut message = MultiHopMessage::new(source, destination, signal);
let next_hop = router.route_message(&message).await?;
if let Some(hop) = next_hop {
    message.forward(current_node)?;
    emit_on_channel(&hop.channel, &message.signal).await?;
}
```

### Route Discovery
```rust
let discovery = router.discover_route(destination).await;
// Discovery request propagates through network
// Reply contains discovered path
```

## Real-World Applications

### 1. IoT Sensor Networks
Sensors relay data through neighbors to reach a central gateway, extending coverage beyond direct radio range.

### 2. Disaster Response
Emergency devices form ad-hoc networks, routing messages through any available path when infrastructure fails.

### 3. Swarm Robotics
Robots coordinate by passing commands and state updates through the swarm, even when not all robots can communicate directly.

### 4. Mesh Lighting
Smart lights relay control signals across a building, creating a self-organizing network without central infrastructure.

## Performance Characteristics

- **Routing overhead**: ~100-500µs per hop (depends on channel)
- **Memory per route**: ~100-200 bytes
- **Route cache size**: Configurable (default: 3 routes per pair)
- **Discovery latency**: Proportional to network diameter

## Next Steps

To extend this demo:

1. **Dynamic routing**: Implement reactive route discovery
2. **Link quality**: Add signal strength monitoring
3. **Multiple paths**: Enable multipath routing for reliability
4. **Network simulation**: Test larger topologies (10+ nodes)
5. **Real hardware**: Deploy on physical devices with actual sensors

## Related Components

- `crates/signal/src/routing.rs`: Core routing implementation
- `crates/signal/src/emitter.rs`: Physical signal emission
- `crates/grid/src/relay.rs`: Grid-level relay mesh
- `examples/relay-demo/`: Grid relay demonstration

## Architecture Notes

### Design Principles

1. **Layered approach**: 
   - Signal layer handles physical transmission
   - Grid layer manages node discovery and handshake
   - Routing layer bridges the two

2. **Protocol flexibility**:
   - TTL prevents loops
   - Hop counting limits path length
   - Route recording enables diagnostics

3. **Quality-based selection**:
   - Success rate (60% weight)
   - Hop count (30% weight)
   - Route age (10% weight)

### Future Enhancements

- **AODV-style discovery**: On-demand route creation
- **Link reversal**: Handle broken routes dynamically
- **Multicast**: Send to multiple destinations efficiently
- **QoS routing**: Prioritize latency/reliability/energy
