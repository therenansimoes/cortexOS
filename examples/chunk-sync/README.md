# Event Chunk Sync Example

This example demonstrates the event chunk synchronization functionality in CortexOS.

## Features Demonstrated

1. **Event Chunking**: Breaking down events into manageable chunks
2. **Delta Sync**: Identifying and syncing only missing chunks between peers
3. **Bandwidth Throttling**: Controlling sync speed to avoid network congestion
4. **Progress Tracking**: Monitoring sync operations in real-time
5. **Hash Verification**: Ensuring data integrity during transfer

## Running the Example

```bash
cargo run --package chunk-sync
```

## What the Example Does

### Part 1: Basic Chunk Sync
- Creates Node 1 with 25 events
- Chunks the events into groups of 5
- Node 2 starts with only 2 chunks (simulating partial state)
- Computes the delta (missing chunks)
- Syncs only the missing 3 chunks from Node 1 to Node 2

### Part 2: Progress Tracking
- Demonstrates how to track sync progress
- Shows percentage completion, chunks synced, bytes transferred, and elapsed time

## Key Components

- **EventChunkStore**: Manages event chunking and serialization
- **EventChunkSyncManager**: Coordinates chunk transfers with bandwidth throttling
- **DeltaSyncProtocol**: Computes differences and requests only missing data

## Output

The example shows:
- Number of chunks created
- Hash of each chunk
- Progress of chunk synchronization
- Final sync statistics (bytes transferred, duration, etc.)

## Use Cases

This synchronization pattern is useful for:
- Distributed event stores across multiple nodes
- Offline-first applications that need to sync when reconnecting
- Bandwidth-constrained environments
- Large-scale event log replication
