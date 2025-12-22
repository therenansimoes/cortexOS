// CortexOS iOS C Header
// Auto-generated bridge for Swift

#ifndef cortex_h
#define cortex_h

#include <stdint.h>
#include <stdbool.h>

// Initialize CortexOS
bool cortex_init(void);

// Get node ID (caller must free with cortex_free_string)
char* cortex_get_node_id(void);

// Free a string allocated by Rust
void cortex_free_string(char* s);

// ============ Agent API ============

// Start a heartbeat agent
char* cortex_start_heartbeat_agent(const char* name, uint64_t interval_secs);

// Start a logger agent
char* cortex_start_logger_agent(const char* name);

// Start a local inference agent
char* cortex_start_inference_agent(const char* name);

// Start a remote inference agent (connects to Ollama, etc.)
char* cortex_start_remote_inference_agent(const char* name, const char* url, const char* model);

// Start a CoreML inference agent (uses native Apple ML)
char* cortex_spawn_coreml_agent(const char* name);

// Register CoreML callback for inference
typedef char* (*CoreMLCallback)(const char* input);
void cortex_register_coreml(CoreMLCallback callback);

// Count agents
int32_t cortex_agent_count(void);

// List all agents as JSON
char* cortex_list_agents(void);

// Stop an agent
bool cortex_stop_agent(const char* agent_id);

// Remove an agent
bool cortex_remove_agent(const char* agent_id);

// Export agent's conversation history as JSONL dataset
char* cortex_export_dataset(const char* agent_id);

// ============ Messaging API ============

// Send message to a specific agent
char* cortex_send_to_agent(const char* agent_id, const char* message);

// Publish event to all agents
char* cortex_publish_event(const char* kind, const char* payload);

// ============ Discovery API ============

// Broadcast discovery to local network
char* cortex_broadcast_discovery(void);

// ============ Stats API ============

// Get overall stats as JSON
char* cortex_get_stats(void);

// Get event log as JSON array
char* cortex_get_event_log(void);

#endif /* cortex_h */

