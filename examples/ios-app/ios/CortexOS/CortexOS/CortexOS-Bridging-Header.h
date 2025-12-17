#ifndef CortexOS_Bridging_Header_h
#define CortexOS_Bridging_Header_h

void cortex_init(void);
char* cortex_start_agent(const char* name);
char* cortex_send_event(const char* agent_id, const char* payload);

#endif
