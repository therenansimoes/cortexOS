# Planner Agent Demo

This example demonstrates the **Planner Agent** - an AI-assisted task planning and orchestration system for CortexOS.

## Overview

The Planner Agent decomposes complex goals into manageable subgoals and coordinates other agents to execute them. It showcases:

- **Goal Decomposition**: Breaking down high-level goals into actionable subgoals
- **Agent Coordination**: Matching subgoals to agents based on their capabilities
- **Task Scheduling**: Managing execution flow and dependencies
- **Progress Monitoring**: Tracking completion status of plans

## Features Demonstrated

### 1. Goal Decomposition
The planner receives a high-level goal like "build a web server" and automatically decomposes it into:
- Design API endpoints
- Implement HTTP server
- Add request routing
- Add error handling
- Write tests

### 2. Capability-Based Matching
Task executor agents register their capabilities (e.g., "design", "implement", "test") and the planner assigns subgoals to matching agents.

### 3. Event-Driven Architecture
All coordination happens through the event bus:
- `planner.plan_request` - Request a new plan
- `planner.plan_created` - Plan has been created with subgoals
- `planner.plan_completed` - All subgoals completed
- `task.assigned` - Task assigned to an agent
- `task.completed` - Agent completed a task

## Running the Demo

```bash
# From the repository root
cargo run -p planner-demo
```

## Expected Output

```
🚀 Starting Planner Agent Demo
📋 Initializing agents...
🎯 Requesting plan for: 'build a web server'
📊 Processing planning events...
✅ Plan created!
📝 Subgoals:
   1. Design API endpoints
   2. Implement HTTP server
   3. Add request routing
   4. Add error handling
   5. Write tests
📈 Final Statistics:
   Plans created: 1
   Subgoals generated: 5
   Coordination events: 5
```

## Architecture

### Planner Agent
- Maintains active plans in memory
- Decomposes goals using pattern matching (or LLM when enabled)
- Coordinates with the IntentionManager for task assignment
- Monitors plan progress through intention status

### Task Executor Agents
Simple agents that:
- Register their capabilities on initialization
- Listen for task assignments
- Execute tasks and report completion

### Shared Infrastructure
- **EventBus**: Broadcast communication channel
- **IntentionManager**: Manages goals and their assignments
- **GraphStore**: Persists plans as thought nodes

## LLM Integration (Future)

The planner is designed to support LLM-based planning:

```rust
let planner = PlannerAgent::new()
    .with_llm(true);  // Enable LLM-based decomposition
```

When enabled, the planner will use the inference crate to generate more intelligent and context-aware subgoals based on the input goal.

## Success Metrics

As per the requirements (PR #26), the planner aims for:
- **Planning accuracy: > 85%** - Goals are decomposed into actionable, relevant subgoals
- **Effective coordination** - Agents are matched to tasks based on their capabilities
- **Progress tracking** - Plans are monitored and completed successfully

## Related Crates

- `cortex-agent` - Agent framework, lifecycle, intention management
- `cortex-inference` (future) - LLM integration for intelligent planning
- `cortex-core` - Event system and runtime

## Next Steps

To extend this demo:
1. Implement actual task execution logic in executor agents
2. Add LLM-based goal decomposition
3. Support parallel execution strategies
4. Add dependency tracking between subgoals
5. Implement failure recovery and replanning
