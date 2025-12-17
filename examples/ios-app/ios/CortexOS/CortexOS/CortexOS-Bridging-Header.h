#ifndef CortexOS_Bridging_Header_h
#define CortexOS_Bridging_Header_h

// Initialize the CortexOS runtime
void cortex_init(void);

// Start a local agent (returns agent info string, must free with cortex_free_string)
char* cortex_start_agent(const char* name);

// Send an event to an agent (must free result with cortex_free_string)
char* cortex_send_event(const char* agent_id, const char* payload);

// Get agent status (must free result with cortex_free_string)
char* cortex_agent_status(const char* agent_id);

// Broadcast discovery message (must free result with cortex_free_string)
char* cortex_broadcast_discovery(void);

// Free a string allocated by Rust
void cortex_free_string(char* s);

#endif
