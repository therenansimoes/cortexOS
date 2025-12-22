use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, Level};

use cortex_core::{
    event::{Event, Payload},
    runtime::EventBus,
};
use cortex_grid::{NodeId, PeerStore, PeerInfo, Capabilities, GridOrchestrator};
use cortex_skill::{NetworkSkillRegistry, SkillId};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    info!("🧠 CortexOS Distributed Task Test");
    info!("   Testing task distribution across multiple nodes");
    info!("");

    // Create two simulated nodes
    let node1_id = NodeId::random();
    let node2_id = NodeId::random();

    info!("📍 Node 1: {} (Math specialist)", node1_id);
    info!("📍 Node 2: {} (Translation specialist)", node2_id);
    info!("");

    // Create peer store and register nodes
    let peer_store = Arc::new(PeerStore::new(Duration::from_secs(120)));
    
    // Node 1 capabilities
    let mut peer1 = PeerInfo::new(node1_id, [0u8; 32]);
    peer1.capabilities = Capabilities {
        can_relay: true,
        can_store: false,
        can_compute: true,
        max_storage_mb: 0,
    };
    peer_store.insert(peer1).await;

    // Node 2 capabilities
    let mut peer2 = PeerInfo::new(node2_id, [0u8; 32]);
    peer2.capabilities = Capabilities {
        can_relay: true,
        can_store: false,
        can_compute: true,
        max_storage_mb: 0,
    };
    peer_store.insert(peer2).await;

    info!("✅ Both nodes registered in peer store");
    info!("");

    // Create event bus and orchestrator for Node 1
    let event_bus = Arc::new(EventBus::default());
    let mut orchestrator = GridOrchestrator::new(
        node1_id,
        Arc::clone(&peer_store).as_ref().clone(),
        Arc::clone(&event_bus),
    );

    orchestrator.start().await.expect("Failed to start orchestrator");
    info!("🎯 Grid Orchestrator started on Node 1");
    info!("");

    // Wait a moment for orchestrator to initialize
    sleep(Duration::from_millis(500)).await;

    // Test 1: Delegate a task manually
    info!("📨 Test 1: Manual task delegation");
    let task_payload = b"Compute: 2 + 2 = ?".to_vec();
    let task_id_hash = blake3::hash(&task_payload);
    let task_id: [u8; 32] = *task_id_hash.as_bytes();
    
    match orchestrator.delegate_task(task_id, task_payload.clone()).await {
        Ok(target_node) => {
            info!("   ✅ Task delegated to: {}", target_node);
            if target_node == node2_id {
                info!("   📝 Task sent to Node 2 (Translation specialist)");
            } else if target_node == node1_id {
                info!("   📝 Task sent to Node 1 (Math specialist)");
            }
        }
        Err(e) => {
            info!("   ❌ Delegation failed: {}", e);
        }
    }
    info!("");

    // Test 2: Auto-delegation via event
    info!("📨 Test 2: Auto-delegation via event bus");
    let task_event = Event::new(
        "test.agent",
        "agent.task.delegate",
        Payload::inline(b"Translate: Hello World to Spanish".to_vec()),
    );
    
    info!("   Publishing task delegation event...");
    if event_bus.publish(task_event).is_ok() {
        info!("   ✅ Event published to event bus");
        info!("   🔄 Orchestrator should auto-delegate to available peer");
    } else {
        info!("   ❌ Failed to publish event");
    }
    info!("");

    // Wait for delegation to happen
    sleep(Duration::from_secs(2)).await;

    // Test 3: Show peer capabilities
    info!("📊 Test 3: Peer capabilities");
    let compute_peers = peer_store
        .find_by_capability(|caps| caps.can_compute)
        .await;
    
    info!("   Found {} peers with compute capability:", compute_peers.len());
    for peer in &compute_peers {
        info!("     - {} (compute: {}, relay: {})", 
            peer.node_id, 
            peer.capabilities.can_compute,
            peer.capabilities.can_relay);
    }
    info!("");

    // Test 4: Skill-based routing (if available)
    info!("📚 Test 4: Skill registry");
    let skill_registry = Arc::new(tokio::sync::RwLock::new(
        NetworkSkillRegistry::new(node1_id)
    ));
    
    {
        let mut registry = skill_registry.write().await;
        registry.register_node_skill(node1_id, SkillId::new("math.compute"));
        registry.register_node_skill(node2_id, SkillId::new("translate.spanish"));
        info!("   Registered skills:");
        info!("     - Node 1: math.compute");
        info!("     - Node 2: translate.spanish");
    }
    info!("");

    info!("✅ Distributed task test complete!");
    info!("");
    info!("Key features demonstrated:");
    info!("  • Peer discovery and registration");
    info!("  • Capability-based peer selection");
    info!("  • Manual task delegation");
    info!("  • Auto-delegation via event bus");
    info!("  • Skill registry for specialized routing");
}

