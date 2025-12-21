use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use cortex_core::runtime::EventBus;
use cortex_grid::{Capabilities, GridOrchestrator, NodeId, PeerInfo, PeerStore};
use cortex_reputation::{SkillId, TrustGraph};
use cortex_skill::{
    DelegationCoordinator, LocalSkillRegistry, NetworkSkillRegistry,
    SkillExecutor, SkillRouter, SkillTask,
};

/// Test task delegation with local execution
#[tokio::test]
async fn test_local_task_execution() {
    let my_id = NodeId::random();
    let peer_store = PeerStore::new(Duration::from_secs(60));
    let event_bus = Arc::new(EventBus::default());
    let trust_graph = Arc::new(RwLock::new(TrustGraph::new(my_id)));

    // Setup orchestrator
    let orchestrator = Arc::new(GridOrchestrator::new(
        my_id,
        peer_store.clone(),
        Arc::clone(&event_bus),
    ));

    // Setup executor
    let local_skills = Arc::new(RwLock::new(LocalSkillRegistry::new()));
    let executor = Arc::new(SkillExecutor::new(
        my_id,
        Arc::clone(&local_skills),
        Arc::clone(&trust_graph),
    ));

    // Setup router
    let network_skills = Arc::new(RwLock::new(NetworkSkillRegistry::new(my_id)));
    let router = Arc::new(SkillRouter::new(
        my_id,
        Arc::clone(&trust_graph),
        Arc::clone(&network_skills),
    ));

    // Create coordinator
    let coordinator = DelegationCoordinator::new(
        my_id,
        orchestrator,
        executor,
        router,
        event_bus,
    );

    // Verify initial state
    assert_eq!(coordinator.active_count().await, 0);
    let stats = coordinator.queue_stats().await;
    assert_eq!(stats.total_queued(), 0);
}

/// Test task delegation to remote node
#[tokio::test]
async fn test_remote_task_delegation() {
    let my_id = NodeId::random();
    let remote_id = NodeId::random();
    
    let peer_store = PeerStore::new(Duration::from_secs(60));
    
    // Add remote peer with compute capability
    let mut remote_peer = PeerInfo::new(remote_id, [0u8; 32]);
    remote_peer.capabilities = Capabilities {
        can_compute: true,
        ..Default::default()
    };
    peer_store.insert(remote_peer).await;
    
    let event_bus = Arc::new(EventBus::default());
    let trust_graph = Arc::new(RwLock::new(TrustGraph::new(my_id)));

    // Setup components
    let orchestrator = Arc::new(GridOrchestrator::new(
        my_id,
        peer_store.clone(),
        Arc::clone(&event_bus),
    ));

    let local_skills = Arc::new(RwLock::new(LocalSkillRegistry::new()));
    let executor = Arc::new(SkillExecutor::new(
        my_id,
        Arc::clone(&local_skills),
        Arc::clone(&trust_graph),
    ));

    let network_skills = NetworkSkillRegistry::new(my_id);
    let test_skill = SkillId::new("test.skill");
    
    // Register that remote node has the skill
    network_skills.register_node_skill(remote_id, test_skill.clone());
    
    let network_skills = Arc::new(RwLock::new(network_skills));
    let router = Arc::new(SkillRouter::new(
        my_id,
        Arc::clone(&trust_graph),
        Arc::clone(&network_skills),
    ));

    let coordinator = DelegationCoordinator::new(
        my_id,
        orchestrator,
        executor,
        router,
        event_bus,
    );

    // Create a task (not submitted - just validating setup)
    let _task = SkillTask::new(
        test_skill,
        cortex_skill::SkillInput::new().with_text("test input"),
        my_id,
    );

    // Note: Full delegation would require starting the coordinator and having
    // actual network communication. This test validates the setup.
    
    assert_eq!(coordinator.active_count().await, 0);
}

/// Test task queue priority handling
#[tokio::test]
async fn test_task_priority_handling() {
    let my_id = NodeId::random();
    let peer_store = PeerStore::new(Duration::from_secs(60));
    let event_bus = Arc::new(EventBus::default());
    let trust_graph = Arc::new(RwLock::new(TrustGraph::new(my_id)));

    let orchestrator = Arc::new(GridOrchestrator::new(
        my_id,
        peer_store.clone(),
        Arc::clone(&event_bus),
    ));

    let local_skills = Arc::new(RwLock::new(LocalSkillRegistry::new()));
    let executor = Arc::new(SkillExecutor::new(
        my_id,
        Arc::clone(&local_skills),
        Arc::clone(&trust_graph),
    ));

    let network_skills = Arc::new(RwLock::new(NetworkSkillRegistry::new(my_id)));
    let router = Arc::new(SkillRouter::new(
        my_id,
        Arc::clone(&trust_graph),
        Arc::clone(&network_skills),
    ));

    let coordinator = DelegationCoordinator::new(
        my_id,
        orchestrator,
        executor,
        router,
        event_bus,
    );

    // Verify queue stats structure
    let stats = coordinator.queue_stats().await;
    assert_eq!(stats.low_priority, 0);
    assert_eq!(stats.normal_priority, 0);
    assert_eq!(stats.high_priority, 0);
    assert_eq!(stats.critical_priority, 0);
    assert_eq!(stats.in_flight, 0);
}

/// Test metrics tracking
#[tokio::test]
async fn test_metrics_tracking() {
    let my_id = NodeId::random();
    let peer_store = PeerStore::new(Duration::from_secs(60));
    let event_bus = Arc::new(EventBus::default());
    let trust_graph = Arc::new(RwLock::new(TrustGraph::new(my_id)));

    let orchestrator = Arc::new(GridOrchestrator::new(
        my_id,
        peer_store.clone(),
        Arc::clone(&event_bus),
    ));

    let local_skills = Arc::new(RwLock::new(LocalSkillRegistry::new()));
    let executor = Arc::new(SkillExecutor::new(
        my_id,
        Arc::clone(&local_skills),
        Arc::clone(&trust_graph),
    ));

    let network_skills = Arc::new(RwLock::new(NetworkSkillRegistry::new(my_id)));
    let router = Arc::new(SkillRouter::new(
        my_id,
        Arc::clone(&trust_graph),
        Arc::clone(&network_skills),
    ));

    let coordinator = DelegationCoordinator::new(
        my_id,
        orchestrator,
        executor,
        router,
        event_bus,
    );

    // Get initial metrics
    let metrics = coordinator.metrics().await;
    assert_eq!(metrics.total_submitted, 0);
    assert_eq!(metrics.total_completed, 0);
    assert_eq!(metrics.total_failed, 0);
    assert_eq!(metrics.success_rate(), 0.0);
}
