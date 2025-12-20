# Event Chunk Synchronization

This document describes the event chunk synchronization system in CortexOS, which enables efficient replication of event stores across distributed nodes in the Grid.

## Overview

Event Chunk Sync is a key component of CortexOS's distributed architecture. It allows nodes to:
- Share event logs efficiently across the Grid
- Sync only missing data (delta sync)
- Control bandwidth usage during synchronization
- Track sync progress in real-time
- Ensure data integrity through cryptographic hashing

## Architecture

The system consists of three main components:

### 1. EventChunkStore (`crates/storage/src/chunk_store.rs`)

Manages the chunking and storage of events.

**Key Features:**
- Configurable chunk size (events per chunk)
- BLAKE3 hash computation for integrity
- JSON serialization for network transfer
- Hash-based indexing for fast lookup

**Usage:**
```rust
use cortex_storage::EventChunkStore;

// Create a store with chunks of 100 events each
let store = EventChunkStore::new(100);

// Create chunks from events
let chunks = store.create_chunks(&events)?;

// Retrieve a chunk by hash
let chunk = store.get_chunk(&hash);
```

### 2. EventChunkSyncManager (`crates/grid/src/chunk_sync.rs`)

Coordinates chunk transfers between nodes with bandwidth control.

**Key Features:**
- Bandwidth throttling (configurable bytes/sec)
- In-memory chunk cache
- Hash verification on receive
- Multi-peer sync tracking

**Usage:**
```rust
use cortex_grid::{EventChunkSyncManager, ThrottleConfig};
use std::time::Duration;

let config = ThrottleConfig {
    max_bytes_per_sec: 1_000_000, // 1 MB/s
    window_duration: Duration::from_secs(1),
};

let sync_manager = EventChunkSyncManager::new(node_id, config);

// Handle incoming chunk
sync_manager.handle_chunk_put(hash, data).await?;

// Request chunk
let chunk = sync_manager.handle_chunk_get(hash).await?;
```

### 3. DeltaSyncProtocol (`crates/grid/src/chunk_sync.rs`)

Implements efficient delta synchronization by identifying missing chunks.

**Key Features:**
- Set-based delta computation
- Batch chunk requests
- Progress tracking per peer

**Usage:**
```rust
use cortex_grid::DeltaSyncProtocol;

let delta = DeltaSyncProtocol::new(sync_manager);

// Compute missing chunks
let missing = delta.compute_delta(&local_hashes, &remote_hashes).await;

// Request missing chunks
let chunks = delta.request_missing_chunks(peer_id, missing).await?;
```

## Protocol

### Wire Protocol Messages

The system uses the existing Grid wire protocol messages:

- **EventChunkGet**: Request a chunk by hash
  ```rust
  Message::EventChunkGet { hash: [u8; 32] }
  ```

- **EventChunkPut**: Send a chunk with its hash
  ```rust
  Message::EventChunkPut { 
      hash: [u8; 32], 
      data: Vec<u8> 
  }
  ```

### Sync Flow

1. **Discovery**: Nodes exchange manifest of available chunk hashes
2. **Delta Computation**: Calculate missing chunks (set difference)
3. **Request**: Request missing chunks via EventChunkGet
4. **Transfer**: Receive chunks via EventChunkPut
5. **Verification**: Verify hash matches received data
6. **Storage**: Store verified chunks locally

```
Node A                          Node B
  |                               |
  |-- Get Manifest -------------->|
  |<-- Manifest (hashes) ---------|
  |                               |
  | Compute Delta (missing)       |
  |                               |
  |-- EventChunkGet (hash1) ----->|
  |<-- EventChunkPut (data1) -----|
  |                               |
  | Verify hash1                  |
  | Store chunk1                  |
  |                               |
  |-- EventChunkGet (hash2) ----->|
  |<-- EventChunkPut (data2) -----|
  |                               |
  | Verify hash2                  |
  | Store chunk2                  |
```

## Bandwidth Throttling

The throttling system prevents network congestion by limiting transfer rates.

**Configuration:**
```rust
ThrottleConfig {
    max_bytes_per_sec: 1_000_000,  // 1 MB/s limit
    window_duration: Duration::from_secs(1),  // 1 second window
}
```

**How it works:**
1. Tracks bytes sent in current time window
2. When limit is exceeded, waits until window resets
3. Resets window after duration elapses
4. Set `max_bytes_per_sec = 0` for unlimited

## Progress Tracking

Track sync operations with detailed metrics:

```rust
// Start tracking
sync_manager.start_sync(peer_id, total_chunks);

// Update progress
sync_manager.update_sync_progress(&peer_id, synced, failed, bytes);

// Get current progress
let progress = sync_manager.get_sync_progress(&peer_id)?;
println!("Progress: {:.1}%", progress.progress_percent());

// Complete sync
let final_stats = sync_manager.complete_sync(&peer_id)?;
```

**Progress Metrics:**
- `total_chunks`: Total chunks to sync
- `synced_chunks`: Successfully synced
- `failed_chunks`: Failed transfers
- `bytes_transferred`: Total bytes transferred
- `elapsed()`: Time since sync started

## Security & Integrity

### Hash Verification

All chunks are verified using BLAKE3:
- Fast cryptographic hashing
- 256-bit hashes (collision-resistant)
- Automatic verification on `handle_chunk_put`
- Rejects chunks with mismatched hashes

### Privacy Filtering

Chunks respect event privacy levels:
- Only public/shareable events are chunked for sync
- Privacy filters applied during chunk creation
- See `crates/storage/src/privacy.rs`

## Performance Considerations

### Chunk Size Selection

Choose chunk size based on your use case:

| Chunk Size | Use Case | Pros | Cons |
|------------|----------|------|------|
| Small (10-50) | Real-time sync | Low latency | More overhead |
| Medium (100-500) | Balanced | Good for most cases | - |
| Large (1000+) | Batch sync | Less overhead | Higher latency |

### Caching

The sync manager uses an in-memory cache:
- Reduces redundant network requests
- Configurable memory limits
- Manual cache clearing with `clear_cache()`

### Memory Usage

Monitor cache size:
```rust
let size = sync_manager.cache_size_bytes();
println!("Cache: {} KB", size / 1024);
```

## Example Usage

See `examples/chunk-sync` for a complete working example:

```bash
cargo run --package chunk-sync
```

The example demonstrates:
- Creating 25 events and chunking them
- Delta sync between two nodes
- Progress tracking
- Bandwidth throttling

## Testing

Run the test suite:

```bash
# Grid tests (EventChunkSyncManager, DeltaSyncProtocol)
cargo test --package cortex-grid chunk_sync

# Storage tests (EventChunkStore)
cargo test --package cortex-storage chunk_store
```

## Future Enhancements

Planned improvements:
- [ ] Persistent chunk storage (RocksDB backend)
- [ ] Compression for chunk data
- [ ] Chunk size auto-tuning based on network conditions
- [ ] Parallel chunk transfers
- [ ] Resume interrupted syncs
- [ ] Chunk expiration/cleanup policies

## Related Components

- **Grid Discovery** (`crates/grid/src/discovery.rs`): Find peers to sync with
- **Wire Protocol** (`crates/grid/src/wire.rs`): Message formats
- **Event Store** (`crates/storage/src/event_store.rs`): Event persistence
- **Privacy System** (`crates/storage/src/privacy.rs`): Privacy controls

## References

- [BLAKE3 Hash Function](https://github.com/BLAKE3-team/BLAKE3)
- [Grid Wire Protocol](../crates/grid/src/wire.rs)
- [Event Model](../crates/storage/src/types.rs)
