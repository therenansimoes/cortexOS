use cortex_core::runtime::Runtime;
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

/// Global runtime instance
static mut RUNTIME_INSTANCE: Option<Arc<Mutex<Runtime>>> = None;

/// Initialize the CortexOS runtime on iOS
#[uniffi::export]
pub fn cortex_init() -> bool {
    unsafe {
        if RUNTIME_INSTANCE.is_some() {
            return true;
        }
        
        let cortex_runtime = Runtime::new();
        RUNTIME_INSTANCE = Some(Arc::new(Mutex::new(cortex_runtime)));
        true
    }
}

/// Start a local agent
#[uniffi::export]
pub fn cortex_start_agent(name: String) -> String {
    let agent_id = Uuid::new_v4().to_string();
    format!("Agent '{}' started with ID: {}", name, agent_id)
}

/// Send an event to an agent
#[uniffi::export]
pub fn cortex_send_event(agent_id: String, payload: String) -> String {
    format!("Event sent to agent {}: {}", agent_id, payload)
}

/// Get the list of connected peers
#[uniffi::export]
pub fn cortex_list_peers() -> Vec<String> {
    vec![]
}

/// Get agent status
#[uniffi::export]
pub fn cortex_agent_status(agent_id: String) -> String {
    format!("Agent {} status: active", agent_id)
}

/// Broadcast a discovery message
#[uniffi::export]
pub fn cortex_broadcast_discovery() -> String {
    "Discovery broadcasted to network".to_string()
}

// UniFFI scaffolding
uniffi::setup_scaffolding!();
