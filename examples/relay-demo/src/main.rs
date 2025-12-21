use cortex_grid::{NodeId, RelayNode, RotatingIdentity};
use tracing::{info, Level};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_target(false)
        .init();

    info!("🌐 CortexOS Relay Mesh Demo (AirTag-style)");
    info!("   Demonstrating anonymous message relay across nodes");
    info!("");

    let node_a_id = NodeId::random();
    let node_b_id = NodeId::random();
    let node_c_id = NodeId::random();

    info!("Creating 3 relay nodes:");
    info!("  Node A (sender):   {}", node_a_id);
    info!("  Node B (relay):    {}", node_b_id);
    info!("  Node C (receiver): {}", node_c_id);
    info!("");

    let (node_a, _rx_a) = RelayNode::new(node_a_id);
    let (node_b, mut rx_b) = RelayNode::new(node_b_id);
    let (node_c, _rx_c) = RelayNode::new(node_c_id);

    node_a.start().await.expect("Failed to start node A");
    node_b.start().await.expect("Failed to start node B");
    node_c.start().await.expect("Failed to start node C");

    let receiver_identity = RotatingIdentity::new();
    let receiver_pubkey = *receiver_identity.public_key();

    info!(
        "📡 Receiver's public key hash: {:02x?}",
        receiver_identity.pubkey_hash()
    );
    info!("");

    let secret_message = b"Hello from CortexOS! This is a secret message.";
    info!(
        "📨 Original message: {:?}",
        String::from_utf8_lossy(secret_message)
    );

    let beacon = node_a
        .create_beacon(&receiver_pubkey, secret_message)
        .await
        .expect("Failed to create beacon");

    info!("🔒 Created encrypted beacon:");
    info!("   TTL: {}", beacon.ttl);
    info!("   Hop count: {}", beacon.hop_count);
    info!("   Recipient hash: {:02x?}", beacon.recipient_pubkey_hash);
    info!(
        "   Encrypted size: {} bytes",
        beacon.encrypted_payload.len()
    );
    info!("");

    info!("🔄 Node B receives and forwards the beacon...");
    node_b
        .handle_beacon(beacon.clone())
        .await
        .expect("Failed to handle beacon");

    if let Ok(msg) = rx_b.try_recv() {
        info!("   Node B forwarded message: {:?}", msg);
    }

    info!("");
    info!("📥 Node C (receiver) receives beacon...");

    let node_c_clone = node_c.clone();
    node_c_clone
        .handle_beacon(beacon.clone())
        .await
        .expect("Failed to handle on C");

    info!("");
    info!("🔓 Beacon details:");

    let encrypted = &beacon.encrypted_payload;
    if encrypted.len() > 32 {
        info!("   Ephemeral pubkey: {:02x?}...", &encrypted[..8]);
        info!("   Ciphertext length: {} bytes", encrypted.len() - 32);
    }

    info!("");
    info!("✅ Demo complete!");
    info!("");
    info!("Key concepts demonstrated:");
    info!("  • End-to-end encryption (X25519 + ChaCha20-Poly1305)");
    info!("  • Relay nodes forward without reading content");
    info!("  • TTL/hop count limits propagation");
    info!("  • Rotating identities for privacy");

    node_a.stop().await;
    node_b.stop().await;
    node_c.stop().await;
}
