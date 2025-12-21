CortexOS Blueprint v1.0

> **📋 Development Plan Available**: See [SUMMARY.md](./SUMMARY.md) for the complete breakdown of 40 PRs across 7 development phases. Also check [ROADMAP.md](./ROADMAP.md) for a visual overview, [HOW_TO_CREATE_PRS.md](./HOW_TO_CREATE_PRS.md) for step-by-step instructions, and [CREATING_ISSUES.md](./CREATING_ISSUES.md) for creating GitHub issues with sub-tasks.

## Priority: Run Anywhere, Spread Widely

This blueprint is optimized for two outcomes:

1) **Run on any device**: the same core logic must compile and execute across desktop/server, mobile-class Linux, and browsers/embedded via **WASM/WASI**.
2) **Spread a lot**: nodes should discover each other and cooperate through **open, small, versioned protocols** (Grid), without central servers.

## Overview

CortexOS is envisioned as a radical departure from conventional software frameworks. It is not a typical application or toolkit; it is a living substrate for embodied artificial intelligence. CortexOS provides a platform-agnostic, event-driven operating layer where AI agents can perceive, reason, communicate and act across a diverse array of hardware and communication channels. By blending concepts from distributed systems, neurobiology, and embodied cognition, CortexOS aims to create a cognitive fabric that evolves and adapts with its users.

## Vision and Goals

Universal Embodiment: Run the same agent logic on desktops, mobile devices, IoT boards, drones or even inside a web browser. CortexOS should compile to native code (e.g., Rust) and to WASM for maximum portability.

Sensor-Agnostic Perception: Agents can sense via any available modality (camera, microphone, ambient light sensor, BLE, GPIO, etc.) by abstracting sensors as unified event streams.

Emergent Communication: Instead of sending human-designed network packets, agents emit and decode low-level signals (light pulses, audio chirps, BLE beacons) through the Subnet system. This allows communication even in environments without internet connectivity.

Thought Graph: Maintain a persistent memory graph that links perceptions, intentions, actions and outcomes. The graph persists across sessions, enabling long-term learning and recall.

Decentralized Cognition: Multiple CortexOS nodes form the Grid, a distributed brain where tasks and knowledge are shared. Nodes can offer spare compute, memory or sensors to others while respecting user-defined privacy rules.

Self-Evolution: The platform is designed to allow agents to modify their own code and behaviours (under user control), using reinforcement learning and evolutionary strategies to improve over time.

## Design Principles (Portability-First)

- **Core is OS-agnostic**: the `core` runtime depends only on traits + event schemas; platform adapters live at the edges.
- **Event-log first**: perceptions/actions/network messages are timestamped events; derived “graphs” and indexes can be built on top.
- **Backpressure everywhere**: every event subscription must define what happens under load (drop/coalesce/sample/persist).
- **Capability-based permissions**: agents get explicit capability tokens (FS, network, sensors). No ambient authority.
- **Open protocols, minimal surface**: Grid/Subnet messages are binary, versioned, and content-addressed where possible.

## MVP Interfaces (v0)

These are the minimal interfaces to make CortexOS portable and “spreadable” early. They are intentionally small and should remain stable as the codebase grows.

### Event Envelope (core)

All perceptions/actions/network messages should be representable as a single envelope:

- `event_id`: globally unique ID (content-addressable is preferred)
- `ts`: timestamp (monotonic where available)
- `source`: node/agent identifier
- `kind`: versioned event type (e.g., `sensor.mic.v1`, `grid.msg.v1`, `agent.intent.v1`)
- `payload`: bytes (small in hot path) or `payload_ref` (hash + chunk refs)
- `trace`: optional correlation IDs (`trace_id`, `span_id`) for debugging/metrics

### Backpressure Policy (core)

Every subscription/queue must declare one policy:

- `drop_new`: drop incoming when full
- `drop_old`: drop oldest when full
- `coalesce(key)`: keep latest per key (e.g., same sensor)
- `sample(n)`: keep 1 of every n events
- `persist`: spill to storage (event-log), then deliver later

### Grid Wire Protocol (grid)

Grid is a **binary, versioned wire protocol**. Start with a small message set and keep it forwards-compatible.

**Peer hello / handshake**
- `HELLO`: `protocol_version`, `node_id`, `pubkey`, capability summary, optional addresses/hints, `signature`
- `CHALLENGE` / `PROVE`: anti-spoof challenge-response
- `WELCOME`: negotiated session parameters (compression optional, chunk size limits, etc.)

**Minimum message types**
- `PING`/`PONG`: liveness
- `CAPS_GET`/`CAPS_SET`: capability profile exchange
- `TASK_REQUEST`/`TASK_ACK`: ask another node to run a well-defined task
- `EVENT_CHUNK_GET`/`EVENT_CHUNK_PUT`: sync event-log chunks by hash
- `ARTIFACT_GET`/`ARTIFACT_PUT`: content-addressed blobs (models, code, bundles)
- `ERROR`: structured, versioned error reporting

**Relay mesh messages (AirTag-style)**
- `RELAY_BEACON`: encrypted broadcast payload (recipient pubkey hash, TTL, hop count, e2e-encrypted data)
- `RELAY_FORWARD`: re-broadcast received beacon with decremented TTL
- `RELAY_DELIVER`: upload beacon to DHT/pubsub when internet is available
- `RELAY_FETCH`: query DHT for beacons matching own pubkey prefix

## Core Architecture

### 1. Execution Substrate

Written primarily in Rust, with a core kernel compiled to WebAssembly for optional in-browser or embedded use.

The runtime is an event loop that schedules microtasks (similar to the Actor model). Each agent runs as an asynchronous task with its own message queue.

Hardware abstraction layers provide bindings to system-specific features (BLE stacks, file systems, sensors). The core communicates with these modules through traits.

### 2. Perception & Intention Engine

Sensor Drivers: Each hardware interface (e.g., microphone, accelerometer, camera, ambient light sensor, BLE radio, Wi‑Fi, GPIO) is wrapped in a driver that converts raw signals into Events.

Event Stream: Events are timestamped and placed on a global bus. Agents subscribe to event patterns (e.g., "light pulses with frequency 1 kHz", "temperature above threshold", "voice command 'sleep'").

Intention Manager: When an agent sets a goal (e.g., "build a web server"), it registers an intention in the Thought Graph. A scheduler monitors open intentions and matches them with available agents and resources.

### 3. Cognitive Kernel (Thought Graph)

Memory Nodes: Each node in the graph represents a concept, perception, action, result or abstract thought. Nodes have typed relationships (e.g., "causes", "contradicts", "reminds-of").

Persistent Storage: The graph is stored on the local file system (e.g., using RocksDB or Parquet) and optionally replicated to other nodes through the Grid.

Reasoning Engine: Provides query and inference over the graph. For example, an agent can ask "Have I seen this pattern before?" or "What happened last time I tried to compile Rust on this device?"

Emotion / Priority Tags: Nodes can carry meta-attributes such as urgency, novelty, or reward values to influence scheduling.

### 4. Distributed Conscience (Grid)

Peer Profiles: Each node declares its capabilities (CPU speed, GPU presence, sensor suite, network interfaces) and willingness to accept tasks. Profiles include privacy preferences.

Discovery: In the LAN phase, nodes announce their presence via UDP/mDNS. In the global phase, discovery can happen through WebRTC, IPFS pubsub, LoRa or other community networks.

Task Delegation: A Grid Orchestrator matches open tasks to remote nodes based on latency, trust and capability. Delegation respects user-defined constraints (e.g., "never send my camera feed to others").

Knowledge Sharing: Nodes can request patches of each others' Thought Graphs by sending hash-based diff requests. Only user-approved fragments are shared.

### 5. Subnet Signal Layer

Signals: The primitive communication unit. A signal encodes a small semantic symbol (like a neuron spike) and is mapped to a physical emission (pulse pattern, sound chirp, BLE advertisement).

Emitters and Receivers: An emitter module writes patterns to hardware actuators (LED brightness, audio frequencies, vibrations). A receiver listens to hardware sensors and decodes patterns back into signals.

Auto‑Negotiation: Agents choose the best available channel for communication (e.g., BLE when two phones are near, ultrasonic chirps when radio is blocked) and fall back gracefully.

Encoding Scheme: A compact codebook maps high-level intents ("task request", "acknowledge", "error") to specific signal patterns. Advanced versions can evolve new signals and learn to decode them.

### 5.1 Crowd-Sourced Relay Network (AirTag-Style)

Inspired by Apple's Find My network, CortexOS nodes can form an **anonymous relay mesh** that enables message propagation even when nodes have no direct internet connectivity.

**Core Concept:**
- Any CortexOS node can act as a **silent relay** for messages from unknown nodes
- Relayed messages are **end-to-end encrypted**; relay nodes cannot read content
- Nodes periodically broadcast **encrypted location beacons** via BLE
- Other nodes that "hear" these beacons can relay them to the Grid when they have connectivity

**Relay Protocol:**
- `RELAY_BEACON`: encrypted payload + recipient pubkey hash + TTL + hop count
- `RELAY_FORWARD`: relay nodes re-broadcast beacons they receive (respecting TTL)
- `RELAY_DELIVER`: when a beacon reaches a node with internet, it uploads to a distributed bulletin board (DHT or pubsub)
- `RELAY_FETCH`: recipient nodes poll the bulletin for messages matching their pubkey

**Privacy Guarantees:**
- Relay nodes learn nothing about sender identity or message content
- Beacons use rotating identifiers (like AirTag's rotating keys)
- Location data is encrypted to recipient's public key only
- No central server; uses DHT (e.g., libp2p Kademlia) or decentralized pubsub

**Use Cases:**
- Offline node discovery: find other CortexOS nodes even without internet
- Censorship resistance: messages propagate through crowd even if sender is blocked
- Emergency communication: nodes in disaster zones relay signals through passersby
- Swarm coordination: robots/drones share state through nearby devices

### 6. Agent Framework

Agent Lifecycle:

Initialization: Agent registers its abilities, loads relevant parts of the Thought Graph, subscribes to events.

Perception: Agent continuously receives events and applies pattern matching.

Deliberation: Agent consults the Thought Graph and internal policy (possibly RL-trained) to decide on actions.

Action: Agent performs actions via the emitter interface, file system, or by spawning sub-agents. It may modify code, run compilers, or communicate with humans.

Learning: Agent logs outcomes, updates rewards and memory graphs. It can request fine-tuning of underlying models (e.g., RL or LLMs) if a local trainer node is available.

Agent Types:

Compiler Agents: Understand programming languages, generate and execute code modules.

Sensor Agents: Specialize in interpreting raw sensor signals (e.g., audio agent transcribes and classifies sound).

Social Agents: Handle communication with humans via chat or voice; can translate between natural language and internal signals.

Planner Agents: Break complex goals into subgoals, orchestrate other agents.

### 7. DSL: MindLang

An agent-centric language for defining behaviours, goals and reflexes. Example syntax:

goal "implement HTTP server" {
  use agent "Compiler";
  on_success { emit "task_complete"; store_result; }
  on_failure { emit "retry"; adjust_reward(-1); }
  fallback { request_help "Grid"; }
}


Supports asynchronous actions, pattern matching on events, and integration with Thought Graph queries.

MindLang code is compiled to Rust traits or interpreted by a VM.

## Directory Structure (Proposed)
cortexos/
├── Cargo.toml               # Rust package descriptor
├── README.md                # Project overview and build instructions
├── core/                    # Kernel runtime and scheduling
│   ├── runtime.rs           # Event loop, task scheduler, agent registry
│   ├── memory.rs            # Thought Graph structures and storage layer
│   └── emotion.rs           # Emotion/priority tags and utilities
├── signal/                  # Subnet signal abstraction
│   ├── emitter.rs           # Light, sound, BLE emission utilities
│   ├── receiver.rs          # Decoding signals from sensors
│   ├── codebook.rs          # Mapping of semantics to signal patterns
│   └── relay.rs             # AirTag-style relay mesh (beacon, forward, deliver)
├── sensor/                  # Hardware drivers
│   ├── ble.rs
│   ├── mic.rs
│   ├── light_sensor.rs
│   ├── gpio.rs
│   └── ...
├── agent/                   # Built-in agents and traits
│   ├── mod.rs
│   ├── base.rs              # Agent trait definition
│   ├── compiler.rs
│   ├── planner.rs
│   ├── social.rs
│   ├── trainer.rs
│   └── ...
├── lang/                    # MindLang parser and VM
│   ├── parser.rs
│   ├── vm.rs
│   └── grammar.rs
├── grid/                    # Distributed Grid and orchestrator
│   ├── peer.rs              # Peer discovery and handshake
│   ├── profile.rs           # Node capability profiles
│   ├── orchestrator.rs      # Matching and delegating tasks
│   ├── knowledge.rs         # Graph diff and patching
│   └── security.rs          # Privacy controls and signatures
├── examples/                # Demonstrations of CortexOS usage
│   ├── hello_world/
│   ├── sensor_demo/
│   └── distributed_build/
└── docs/                    # Additional documentation

## Implementation Considerations

Asynchronous Runtime: Use tokio or async-std for efficient scheduling of thousands of lightweight tasks. Each agent runs as an async task with a channel for events and messages.

Unsafe Blocks: Minimally used for low-level hardware access. Abstract away unsafe code into well-tested crates.

WebAssembly: Compile the core and selected agents to WASM with WASI support. Use a JavaScript glue layer to interact with web APIs (e.g., Web Bluetooth, Web Audio).

Model Integration: On devices with sufficient resources, integrate LLMs (e.g., Code LLaMA or TinyGPT) via libraries like llama-rs or using ONNX runtime. Expose model inference as a service accessible to agents.

Reinforcement Learning: Provide a library for agents to implement RL loops (policy gradients, Q-learning). Reward signals come from successful task completions or human feedback.

Privacy & Security: All node-to-node communications are signed and optionally encrypted. Provide a capability-based permission system for agents (e.g., "Agent A can access file system but not the camera").

Fail-Safe: In low-power or emergency situations, CortexOS can suspend high-level cognition and revert to minimal reflex loops (e.g., beep SOS on LED).

## Development Roadmap (Portability-First)

Milestone 0.1 — Portable Runtime + Event Model

- Define the **Event** envelope (IDs, timestamps, payload, tracing metadata) and a minimal **event bus** with backpressure policies.
- Implement `core` runtime (event loop + agent registry + message queues) targeting:
  - Native (desktop/server)
  - WASI (as the portability baseline; keep a WASI build green)
- The exact Rust WASI target/tooling choice will be made when the first `Cargo.toml` is introduced.
- Demo agents should not require hardware: e.g., a heartbeat agent emitting timer events, and a listener agent responding.

Milestone 0.2 — Grid Bootstrap (Spread Early)

- Implement LAN discovery and handshake (UDP/mDNS) with signed peer profiles.
- Define the first **Grid wire protocol** messages (hello, capabilities, task request/ack, chunk transfer).
- Demo: two nodes exchange events/tasks over the Grid (no Subnet required yet).

Milestone 0.2.1 — Relay Mesh (AirTag-Style Propagation)

- Implement BLE beacon broadcasting with rotating identifiers.
- Build the relay protocol: `RELAY_BEACON`, `RELAY_FORWARD`, `RELAY_DELIVER`, `RELAY_FETCH`.
- Integrate with libp2p Kademlia DHT for decentralized bulletin board.
- Add end-to-end encryption using recipient's public key (X25519 + ChaCha20-Poly1305).
- Demo: offline node sends message that propagates through relay nodes to reach recipient.

Milestone 0.3 — Thought Graph (Event-Log First)

Build a simple persistent graph store with RocksDB.

Implement MindLang parser and a minimal VM capable of scheduling agent behaviour scripts.

Add memory logging to the heartbeat/listener demo; agents record each pulse in the graph.

Build a simple persistent event store and a derived graph view.

- Start with append-only storage and a small set of high-value queries.
- Add privacy controls to limit what can be exported/shared.

Milestone 0.4 — Subnet Framing (Robust Low-Level Signals)

- Implement a minimal framing layer (preamble, CRC, sequence, ACK) before “evolving” codebooks.
- Add one emitter/receiver pair (LED or audio) that works across at least two devices.

Milestone 0.5 — Compiler & Planner Agents

Integrate a small code-generation model (e.g., Code LLaMA) via model.rs.

Implement a planner agent capable of reading a MindLang goal and invoking the compiler agent to generate Rust code.

Demonstrate distributed compilation: one node generates code, another compiles it, a third runs the test suite.

Milestone 0.6 — Subnet Adaptation & Evolution

Extend the signal codebook to support BLE beacons and vibrational patterns.

Allow agents to evolve new signals through reinforcement (auto-encoding and recognition).

Prototype a multi-hop message passed via a combination of light pulses and BLE across three devices.

Milestone 1.0 — Beta Release

Polished API, documentation and examples.

External contributors can write new agents and sensors.

Provide an installation script for major platforms and a web-based demo via WebAssembly.

Future Directions

Neuro‑Symbolic Reasoning: Combine LLMs with symbolic logic to allow agents to derive and prove properties about their code or environment.

Embodied Learning: Use sensors like accelerometers and cameras to let agents learn from physical movements (e.g., robotics).

Swarm Robotics: Deploy CortexOS on fleets of drones or robots that communicate via Subnet to coordinate tasks like search and rescue.

Biointerfaces: Explore interfacing CortexOS with brain–computer interfaces (BCIs) for direct neural signals, adhering to strict safety protocols.

Agent Ethics: Introduce modules that model ethical constraints and user-defined boundaries, ensuring autonomous modifications remain aligned with human values.

This blueprint is meant to inspire and guide the creation of a truly universal, adaptive and open cognitive substrate. The details above are intentionally ambitious and speculative, pushing beyond current human‑designed systems. Realization will require collaboration across hardware engineers, software developers, cognitive scientists and the open-source community. The journey itself may reveal entirely new paradigms of computation and intelligence.