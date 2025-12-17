#ifndef CortexOS_Bridging_Header_h
#define CortexOS_Bridging_Header_h

#include <stdbool.h>

// ============================================
// CortexOS iOS FFI - Zero Mock Policy
// All functions call real Rust implementations
// ============================================

// Initialize the CortexOS runtime (returns true on success)
bool cortex_init(void);

// Get local node ID (must free with cortex_free_string)
char* cortex_get_node_id(void);

// Start a HeartbeatAgent - emits periodic heartbeat events
// Returns JSON with agent info (must free with cortex_free_string)
char* cortex_start_heartbeat_agent(const char* name, unsigned long interval_secs);

// Start a LoggerAgent - logs all events it receives
// Returns JSON with agent info (must free with cortex_free_string)
char* cortex_start_logger_agent(const char* name);

// Start an InferenceAgent - processes messages with AI inference
// Returns JSON with agent info (must free with cortex_free_string)
char* cortex_start_inference_agent(const char* name);

// Start a Remote Inference Agent (Ollama/HTTP)
// Returns JSON with agent info (must free with cortex_free_string)
char* cortex_start_remote_inference_agent(const char* name, const char* url, const char* model);

// Get number of running agents
int cortex_agent_count(void);

// List all agents as JSON array (must free with cortex_free_string)
char* cortex_list_agents(void);

// Publish an event to the event bus
// All agents subscribed to this event type will receive it
// Returns JSON result (must free with cortex_free_string)
char* cortex_publish_event(const char* kind, const char* payload);

// Send a message directly to an agent and get response
// Returns JSON with response (must free with cortex_free_string)
char* cortex_send_to_agent(const char* agent_id, const char* message);

// Stop an agent by ID
bool cortex_stop_agent(const char* agent_id);

// Remove an agent completely
bool cortex_remove_agent(const char* agent_id);

// Export agent dataset as JSONL (for fine-tuning)
// Returns JSONL string (must free with cortex_free_string)
char* cortex_export_dataset(const char* agent_id);

// Broadcast LAN discovery
// Returns JSON result (must free with cortex_free_string)
char* cortex_broadcast_discovery(void);

// CoreML Support
typedef char* (*CoreMLCallback)(const char* input);
void cortex_register_coreml(CoreMLCallback callback);
char* cortex_spawn_coreml_agent(const char* name);

// Get runtime stats as JSON (must free with cortex_free_string)
char* cortex_get_stats(void);

// Get event log as JSON array (must free with cortex_free_string)
char* cortex_get_event_log(void);

// Free a string allocated by Rust
void cortex_free_string(char* s);

#endif
