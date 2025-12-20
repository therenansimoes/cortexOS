use cortex_grid::{EventChunkSyncManager, DeltaSyncProtocol, NodeId, ThrottleConfig};
use cortex_storage::{Event, EventChunkStore, Timestamp};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Event Chunk Sync Demo ===\n");

    // Create two nodes
    let node1_id = NodeId::random();
    let node2_id = NodeId::random();

    println!("Node 1: {}", node1_id);
    println!("Node 2: {}\n", node2_id);

    // Node 1: Create some events and chunk them
    println!("Node 1: Creating events...");
    let events: Vec<Event> = (0..25)
        .map(|i| Event {
            id: cortex_storage::EventId::new(),
            kind: format!("test.event.{}", i),
            source: "node1".to_string(),
            timestamp: Timestamp::now(),
            payload: serde_json::json!({
                "message": format!("Event #{}", i),
                "value": i * 10,
            }),
            privacy: cortex_storage::PrivacyLevel::Public,
        })
        .collect();

    println!("Node 1: Created {} events", events.len());

    // Create chunk store with chunk size of 5
    let chunk_store = EventChunkStore::new(5);
    let chunks = chunk_store.create_chunks(&events)?;

    println!("Node 1: Created {} chunks\n", chunks.len());

    // Display chunk info
    for (i, (hash, chunk_events)) in chunks.iter().enumerate() {
        println!(
            "  Chunk {}: {} events, hash: {:02x?}...",
            i,
            chunk_events.len(),
            &hash[..4]
        );
    }
    println!();

    // Setup sync manager for both nodes
    let throttle_config = ThrottleConfig {
        max_bytes_per_sec: 1_000_000, // 1 MB/s
        window_duration: Duration::from_secs(1),
    };

    let node1_sync = Arc::new(EventChunkSyncManager::new(node1_id, throttle_config.clone()));
    let node2_sync = Arc::new(EventChunkSyncManager::new(node2_id, throttle_config));

    // Node 1 stores all chunks
    println!("Node 1: Storing chunks in sync manager...");
    for (hash, _chunk_events) in &chunks {
        let serialized = chunk_store.serialize_chunk(hash)?;
        node1_sync.handle_chunk_put(*hash, serialized).await?;
    }
    println!("Node 1: {} chunks stored\n", node1_sync.cache_size_bytes() / 1024);

    // Node 2 has some chunks (simulate partial sync state)
    println!("Node 2: Simulating partial state (first 2 chunks)...");
    for (hash, _chunk_events) in chunks.iter().take(2) {
        let serialized = chunk_store.serialize_chunk(hash)?;
        node2_sync.handle_chunk_put(*hash, serialized).await?;
    }

    // Get hashes from both nodes
    let node1_hashes: Vec<_> = chunks.iter().map(|(hash, _)| *hash).collect();
    let node2_hashes: Vec<_> = chunks.iter().take(2).map(|(hash, _)| *hash).collect();

    println!("Node 1: {} chunks", node1_hashes.len());
    println!("Node 2: {} chunks\n", node2_hashes.len());

    // Compute delta
    println!("Computing delta sync...");
    let delta_protocol = DeltaSyncProtocol::new(node2_sync.clone());
    let missing_hashes = delta_protocol
        .compute_delta(&node2_hashes, &node1_hashes)
        .await;

    println!("Missing chunks: {}", missing_hashes.len());
    println!();

    // Simulate sync: Node 2 requests missing chunks from Node 1
    println!("Node 2: Syncing missing chunks from Node 1...");
    for hash in &missing_hashes {
        // Node 1 provides the chunk
        let chunk_data = node1_sync
            .handle_chunk_get(*hash)
            .await?
            .ok_or("Chunk not found")?;

        println!(
            "  Syncing chunk {:02x?}... ({} bytes)",
            &hash[..4],
            chunk_data.len()
        );

        // Node 2 receives and stores the chunk
        node2_sync.handle_chunk_put(*hash, chunk_data).await?;
    }

    println!();
    println!("Sync complete!");
    println!("Node 2 cache size: {} KB", node2_sync.cache_size_bytes() / 1024);

    // Verify node 2 now has all chunks
    let final_count = missing_hashes.len() + node2_hashes.len();
    println!("Node 2 total chunks: {}", final_count);
    println!();

    // Progress tracking demo
    println!("=== Progress Tracking Demo ===\n");

    let node3_id = NodeId::random();
    let sync_manager = EventChunkSyncManager::new(
        node3_id,
        ThrottleConfig {
            max_bytes_per_sec: 500_000, // 500 KB/s
            window_duration: Duration::from_secs(1),
        },
    );

    // Start a sync operation
    sync_manager.start_sync(node1_id, 10);

    // Simulate progress updates
    for _i in 0..5 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        sync_manager.update_sync_progress(&node1_id, 2, 0, 2048);

        if let Some(progress) = sync_manager.get_sync_progress(&node1_id) {
            println!(
                "Progress: {:.1}% ({}/{} chunks, {} bytes, {:?} elapsed)",
                progress.progress_percent(),
                progress.synced_chunks,
                progress.total_chunks,
                progress.bytes_transferred,
                progress.elapsed()
            );
        }
    }

    // Complete sync
    if let Some(final_progress) = sync_manager.complete_sync(&node1_id) {
        println!("\nSync completed!");
        println!("  Total chunks: {}", final_progress.total_chunks);
        println!("  Synced: {}", final_progress.synced_chunks);
        println!("  Failed: {}", final_progress.failed_chunks);
        println!("  Bytes: {}", final_progress.bytes_transferred);
        println!("  Duration: {:?}", final_progress.elapsed());
    }

    println!("\n=== Demo Complete ===");

    Ok(())
}
