# Distributed Compilation Demo

This example demonstrates how CortexOS enables distributed compilation across a P2P network using AI agents.

## Overview

This demo showcases a multi-node compilation workflow where:

1. **Planner Node** - Orchestrates the compilation task and coordinates other nodes
2. **Compiler Node** - Performs the actual code compilation
3. **Executor Node** - Validates and tests the compiled artifacts

Each node operates independently with specialized capabilities, communicating through the Grid protocol.

## Architecture

```
┌─────────────┐
│   Planner   │ Creates compilation plan
│    Node     │ Delegates tasks to Grid
└──────┬──────┘
       │
       │ task.compile event
       ▼
┌─────────────┐
│  Compiler   │ Receives compilation task
│    Node     │ Compiles source code
└──────┬──────┘
       │
       │ compilation.result event
       ▼
┌─────────────┐     ┌─────────────┐
│  Executor   │◄────┤   Planner   │ Receives result
│    Node     │     │    Node     │ Monitors completion
└─────────────┘     └─────────────┘
```

## Capabilities

Each node declares its capabilities to enable intelligent task routing:

- **Planner**: `planner`, `coordinator`
- **Compiler**: `compiler.rust`, `compiler.wasm`
- **Executor**: `executor.wasm`, `executor.test`

In a production system, the GridOrchestrator would match tasks to nodes based on these capabilities.

## Running the Demo

```bash
cargo run --bin distributed-compilation
```

## Expected Output

The demo will show:

1. Node initialization with unique IDs
2. Planner creating a compilation task
3. Compiler receiving and processing the task
4. Compilation results being distributed
5. Executor validating the compiled output

## Key Concepts

### Event-Driven Communication

Nodes communicate through typed events on the Grid:

- `task.compile` - Compilation request
- `compilation.result` - Compilation outcome

### Distributed Coordination

The planner demonstrates task decomposition and delegation:

```rust
let task = CompilationTask {
    task_id: "task-001",
    source_code: SAMPLE_CODE,
    language: "rust",
    target: "wasm32-wasi",
};
```

### Simulated Distribution

In this demo, all nodes run in the same process for simplicity. In production:

- Each node would be a separate process or machine
- Nodes would discover each other via libp2p
- GridOrchestrator would handle task routing
- Tasks would be serialized and sent over the network

## Future Enhancements

This demo lays groundwork for:

1. **Real distributed compilation** - Split nodes across machines
2. **Load balancing** - Route tasks based on node availability
3. **Caching** - Share compilation artifacts via Grid
4. **Incremental builds** - Coordinate partial recompilation
5. **Trust & reputation** - Track node reliability

## Related Components

- `crates/agent/` - Agent framework with CompilerAgent and PlannerAgent
- `crates/grid/` - P2P networking and orchestration
- `crates/core/` - Event bus and runtime

## Milestone

This demo fulfills **Milestone 0.5: Compiler & Planner** requirements:

- ✅ Planner agent decomposes goals into tasks
- ✅ Compiler agent handles code compilation
- ✅ Distributed coordination across multiple nodes
- ✅ Event-driven communication via Grid protocol
- ✅ Capability-based task routing (simulated)
