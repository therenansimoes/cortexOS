# Task Delegation System

The Task Delegation System enables distributed task execution across the CortexOS P2P network. It allows nodes to delegate computationally intensive or specialized tasks to other nodes in the Grid based on their capabilities, trust scores, and availability.

## Architecture

The task delegation system consists of several key components:

### 1. Task Queue (Grid Layer)

**Location**: `crates/grid/src/task_queue.rs`

A priority-based queue manager for handling pending tasks:

- **Priority Levels**: Critical, High, Normal, Low
- **Queue Management**: FIFO within each priority level
- **In-flight Tracking**: Monitors tasks currently being executed
- **Timeout Handling**: Automatically removes expired tasks
- **Backpressure**: Configurable maximum queue size per priority

**Key Features**:
```rust
// Create a task queue
let queue = TaskQueue::new(max_size);

// Enqueue with priority
let task = QueuedTask {
    task_id: [1u8; 32],
    payload: vec![],
    priority: TaskPriority::High,
    target_node: Some(node_id),
    retries: 0,
    created_at: Instant::now(),
};
queue.enqueue(task).await;

// Dequeue highest priority task
let next_task = queue.dequeue().await;

// Mark as completed
queue.complete(&task_id).await;

// Or retry on failure
queue.fail(&task_id, true).await;
```

### 2. Metrics Tracker (Grid Layer)

**Location**: `crates/grid/src/metrics.rs`

Tracks task execution performance across the network:

- **Global Metrics**: Total submitted, completed, failed, timed out
- **Execution Times**: Min, max, and average execution time
- **Per-Node Metrics**: Success rates and performance per remote node
- **Success Rates**: Calculated success and failure rates

**Key Features**:
```rust
let tracker = MetricsTracker::new();

// Record task lifecycle
tracker.record_submitted(node_id).await;
tracker.record_completed(node_id, duration).await;
tracker.record_failed(node_id).await;

// Get metrics snapshot
let metrics = tracker.snapshot().await;
println!("Success rate: {:.2}%", metrics.success_rate() * 100.0);
println!("Avg execution time: {}ms", metrics.avg_execution_time_ms);
```

### 3. Delegation Coordinator (Skill Layer)

**Location**: `crates/skill/src/delegation.rs`

Orchestrates task delegation between the Grid and Skill layers:

- **Task Routing**: Uses SkillRouter to find the best node for execution
- **Local vs Remote**: Automatically executes locally when appropriate
- **Queue Management**: Integrates TaskQueue for pending tasks
- **Result Handling**: Collects and publishes task results
- **Metrics Integration**: Tracks all task executions

**Key Features**:
```rust
// Create coordinator
let coordinator = DelegationCoordinator::new(
    my_id,
    orchestrator,
    executor,
    router,
    event_bus,
);

// Start the coordinator
coordinator.start().await?;

// Submit a task
let task = SkillTask::new(skill_id, input, requester_id);
let task_id = coordinator.submit_task(task).await?;

// Get result when ready
let result = coordinator.get_result(&task_id).await;

// Check metrics
let metrics = coordinator.metrics().await;
let stats = coordinator.queue_stats().await;
```

## Task Lifecycle

1. **Submission**: Task is submitted to the coordinator
2. **Routing**: Router selects the best node based on:
   - Node capabilities and skill availability
   - Trust scores and reputation
   - Historical performance metrics
3. **Queueing**: Task is added to priority queue
4. **Execution**: 
   - If local: Execute directly with SkillExecutor
   - If remote: Delegate via GridOrchestrator
5. **Completion**: Result is collected and metrics updated
6. **Retry**: Failed tasks can be retried with backoff

## Integration Points

### Grid Orchestrator

The existing `GridOrchestrator` handles network-level task delegation:
- Sends `TaskRequest` messages to remote nodes
- Receives `TaskAck` responses
- Manages task timeout and retries
- Publishes events to the event bus

### Skill Router

Determines the optimal node for task execution:
- Queries trust graph for node reputation
- Checks skill registry for capability matches
- Calculates routing scores based on trust + skill rating
- Provides fallback options for failed nodes

### Skill Executor

Executes tasks locally:
- Looks up registered skills
- Validates task inputs
- Executes skill logic
- Reports results with timing data

## Configuration

### Task Queue Configuration

```rust
// Maximum tasks per priority queue
const MAX_QUEUE_SIZE: usize = 1000;

// Task timeout (5 minutes default)
const TASK_TIMEOUT_SECS: u64 = 300;
```

### Priority Selection

Task priority affects queue position:
- **Critical**: Infrastructure, security-critical tasks
- **High**: User-facing, time-sensitive tasks
- **Normal**: Background processing, batch jobs
- **Low**: Non-urgent, cleanup tasks

## Metrics and Monitoring

Track system health via metrics:

```rust
let metrics = coordinator.metrics().await;

// Overall health
println!("Total tasks: {}", metrics.total_submitted);
println!("Success rate: {:.1}%", metrics.success_rate() * 100.0);
println!("Failure rate: {:.1}%", metrics.failure_rate() * 100.0);

// Performance
println!("Avg latency: {}ms", metrics.avg_execution_time_ms);
println!("Min latency: {}ms", metrics.min_execution_time_ms);
println!("Max latency: {}ms", metrics.max_execution_time_ms);

// Per-node performance
for (node, node_metrics) in &metrics.per_node {
    println!("Node {}: {:.1}% success, {}ms avg",
        node,
        node_metrics.success_rate() * 100.0,
        node_metrics.avg_execution_time_ms
    );
}
```

## Error Handling

The system handles various failure modes:

- **Queue Full**: Returns `SkillError::QueueFull`
- **No Capable Node**: Returns `SkillError::NoCapableNode`
- **Delegation Failed**: Returns `SkillError::DelegationFailed`
- **Task Timeout**: Automatically cleaned up after timeout
- **Network Errors**: Retries with exponential backoff (up to MAX_RETRIES)

## Testing

### Unit Tests

Each component has comprehensive unit tests:
- `task_queue::tests`: Queue operations, priority handling
- `metrics::tests`: Metric tracking and calculations
- `delegation::tests`: Coordinator setup and state

### Integration Tests

Integration tests validate end-to-end flows:
- `delegation_tests::test_local_task_execution`: Local execution path
- `delegation_tests::test_remote_task_delegation`: Remote delegation
- `delegation_tests::test_task_priority_handling`: Priority queue behavior
- `delegation_tests::test_metrics_tracking`: Metrics collection

Run tests:
```bash
cargo test --package cortex-skill --package cortex-grid
```

## Future Enhancements

Planned improvements:
- Task result caching and deduplication
- Dynamic priority adjustment based on queue depth
- Load-based routing (avoid overloaded nodes)
- Task chaining and dependency management
- Progress streaming for long-running tasks
- Advanced retry strategies (exponential backoff, circuit breaker)

## Example Usage

Complete example:

```rust
use cortex_skill::{DelegationCoordinator, SkillTask, SkillInput};
use cortex_reputation::SkillId;

// Setup (see architecture section for full setup)
let coordinator = setup_coordinator().await;
coordinator.start().await?;

// Submit a task
let skill = SkillId::new("image.resize".to_string());
let input = SkillInput::json(serde_json::json!({
    "url": "https://example.com/image.jpg",
    "width": 800,
    "height": 600
}));

let task = SkillTask::new(skill, input, my_node_id)
    .with_priority(150)  // High priority (128-191 range)
    .with_timeout(60)  // 1 minute timeout
    .with_min_trust(0.5);  // Require 50% trust

let task_id = coordinator.submit_task(task).await?;

// Wait for result
tokio::time::sleep(Duration::from_secs(5)).await;

if let Some(result) = coordinator.get_result(&task_id).await {
    if result.success {
        println!("Task completed in {}ms", result.duration_ms);
        // Process output
    } else {
        eprintln!("Task failed: {}", result.error.unwrap());
    }
}

// Check overall performance
let metrics = coordinator.metrics().await;
println!("System-wide success rate: {:.1}%", 
    metrics.success_rate() * 100.0);
```

## See Also

- [Grid Wire Protocol](../grid/wire.rs) - Network message formats
- [Skill Framework](../skill/README.md) - Skill definition and execution
- [Reputation System](../reputation/README.md) - Trust graph and scoring
