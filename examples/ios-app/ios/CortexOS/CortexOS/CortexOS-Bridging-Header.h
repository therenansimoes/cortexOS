#ifndef CortexOS_Bridging_Header_h
#define CortexOS_Bridging_Header_h

#include <stdbool.h>

// Initialize the CortexOS runtime
void cortex_init(void);

// Get local node ID (must free with cortex_free_string)
char* cortex_get_node_id(void);

// Start a local agent, returns agent ID (must free with cortex_free_string)
char* cortex_start_agent(const char* name);

// Get number of active agents
int cortex_agent_count(void);

// List all agents as JSON (must free with cortex_free_string)
char* cortex_list_agents(void);

// Send an event to an agent (must free result with cortex_free_string)
char* cortex_send_event(const char* agent_id, const char* payload);

// Stop an agent
bool cortex_stop_agent(const char* agent_id);

// Remove an agent
bool cortex_remove_agent(const char* agent_id);

// Get agent status as JSON (must free result with cortex_free_string)
char* cortex_agent_status(const char* agent_id);

// Broadcast discovery message, returns JSON (must free result with cortex_free_string)
char* cortex_broadcast_discovery(void);

// Get runtime stats as JSON (must free with cortex_free_string)
char* cortex_get_stats(void);

// Free a string allocated by Rust
void cortex_free_string(char* s);

#endif
