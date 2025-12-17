use cortex_core::runtime::Runtime;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

/// Agent info stored in the runtime
#[derive(Clone)]
struct AgentInfo {
    id: String,
    name: String,
    status: AgentStatus,
    events_received: u32,
}

#[derive(Clone, Copy, PartialEq)]
enum AgentStatus {
    Active,
    Paused,
    Stopped,
}

impl AgentStatus {
    fn as_str(&self) -> &'static str {
        match self {
            AgentStatus::Active => "active",
            AgentStatus::Paused => "paused",
            AgentStatus::Stopped => "stopped",
        }
    }
}

/// Global state
struct CortexState {
    runtime: Runtime,
    agents: HashMap<String, AgentInfo>,
    discovery_count: u32,
    node_id: String,
}

static mut CORTEX_STATE: Option<Arc<Mutex<CortexState>>> = None;

/// Initialize the CortexOS runtime on iOS
#[no_mangle]
pub extern "C" fn cortex_init() {
    unsafe {
        if CORTEX_STATE.is_some() {
            return;
        }
        
        let state = CortexState {
            runtime: Runtime::new(),
            agents: HashMap::new(),
            discovery_count: 0,
            node_id: Uuid::new_v4().to_string()[..8].to_string(),
        };
        CORTEX_STATE = Some(Arc::new(Mutex::new(state)));
    }
}

/// Get the local node ID
#[no_mangle]
pub extern "C" fn cortex_get_node_id() -> *mut c_char {
    let node_id = unsafe {
        CORTEX_STATE
            .as_ref()
            .map(|s| s.lock().unwrap().node_id.clone())
            .unwrap_or_else(|| "unknown".to_string())
    };
    string_to_c_str(node_id)
}

/// Helper to convert C string to Rust String
unsafe fn c_str_to_string(s: *const c_char) -> String {
    if s.is_null() {
        return String::new();
    }
    CStr::from_ptr(s).to_string_lossy().into_owned()
}

/// Helper to convert Rust String to C string (caller must free with cortex_free_string)
fn string_to_c_str(s: String) -> *mut c_char {
    CString::new(s).unwrap_or_default().into_raw()
}

/// Free a string allocated by Rust
#[no_mangle]
pub extern "C" fn cortex_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

/// Start a local agent - returns agent ID
#[no_mangle]
pub extern "C" fn cortex_start_agent(name: *const c_char) -> *mut c_char {
    let name = unsafe { c_str_to_string(name) };
    let agent_id = Uuid::new_v4().to_string()[..8].to_string();
    
    let agent = AgentInfo {
        id: agent_id.clone(),
        name: name.clone(),
        status: AgentStatus::Active,
        events_received: 0,
    };
    
    unsafe {
        if let Some(state) = CORTEX_STATE.as_ref() {
            let mut s = state.lock().unwrap();
            s.agents.insert(agent_id.clone(), agent);
        }
    }
    
    string_to_c_str(agent_id)
}

/// Get number of active agents
#[no_mangle]
pub extern "C" fn cortex_agent_count() -> i32 {
    unsafe {
        CORTEX_STATE
            .as_ref()
            .map(|s| s.lock().unwrap().agents.len() as i32)
            .unwrap_or(0)
    }
}

/// List all agents as JSON array
#[no_mangle]
pub extern "C" fn cortex_list_agents() -> *mut c_char {
    let json = unsafe {
        CORTEX_STATE
            .as_ref()
            .map(|s| {
                let state = s.lock().unwrap();
                let agents: Vec<String> = state
                    .agents
                    .values()
                    .map(|a| {
                        format!(
                            r#"{{"id":"{}","name":"{}","status":"{}","events":{}}}"#,
                            a.id, a.name, a.status.as_str(), a.events_received
                        )
                    })
                    .collect();
                format!("[{}]", agents.join(","))
            })
            .unwrap_or_else(|| "[]".to_string())
    };
    string_to_c_str(json)
}

/// Send an event to an agent
#[no_mangle]
pub extern "C" fn cortex_send_event(agent_id: *const c_char, payload: *const c_char) -> *mut c_char {
    let agent_id = unsafe { c_str_to_string(agent_id) };
    let payload = unsafe { c_str_to_string(payload) };
    
    let result = unsafe {
        CORTEX_STATE
            .as_ref()
            .map(|s| {
                let mut state = s.lock().unwrap();
                if let Some(agent) = state.agents.get_mut(&agent_id) {
                    agent.events_received += 1;
                    format!("Event delivered to '{}': {}", agent.name, payload)
                } else {
                    format!("Agent {} not found", agent_id)
                }
            })
            .unwrap_or_else(|| "Runtime not initialized".to_string())
    };
    string_to_c_str(result)
}

/// Stop an agent
#[no_mangle]
pub extern "C" fn cortex_stop_agent(agent_id: *const c_char) -> bool {
    let agent_id = unsafe { c_str_to_string(agent_id) };
    
    unsafe {
        CORTEX_STATE
            .as_ref()
            .map(|s| {
                let mut state = s.lock().unwrap();
                if let Some(agent) = state.agents.get_mut(&agent_id) {
                    agent.status = AgentStatus::Stopped;
                    true
                } else {
                    false
                }
            })
            .unwrap_or(false)
    }
}

/// Remove an agent
#[no_mangle]
pub extern "C" fn cortex_remove_agent(agent_id: *const c_char) -> bool {
    let agent_id = unsafe { c_str_to_string(agent_id) };
    
    unsafe {
        CORTEX_STATE
            .as_ref()
            .map(|s| {
                let mut state = s.lock().unwrap();
                state.agents.remove(&agent_id).is_some()
            })
            .unwrap_or(false)
    }
}

/// Get agent status
#[no_mangle]
pub extern "C" fn cortex_agent_status(agent_id: *const c_char) -> *mut c_char {
    let agent_id = unsafe { c_str_to_string(agent_id) };
    
    let result = unsafe {
        CORTEX_STATE
            .as_ref()
            .map(|s| {
                let state = s.lock().unwrap();
                if let Some(agent) = state.agents.get(&agent_id) {
                    format!(
                        r#"{{"id":"{}","name":"{}","status":"{}","events":{}}}"#,
                        agent.id, agent.name, agent.status.as_str(), agent.events_received
                    )
                } else {
                    r#"{"error":"not_found"}"#.to_string()
                }
            })
            .unwrap_or_else(|| r#"{"error":"not_initialized"}"#.to_string())
    };
    string_to_c_str(result)
}

/// Broadcast a discovery message
#[no_mangle]
pub extern "C" fn cortex_broadcast_discovery() -> *mut c_char {
    let result = unsafe {
        CORTEX_STATE
            .as_ref()
            .map(|s| {
                let mut state = s.lock().unwrap();
                state.discovery_count += 1;
                format!(
                    r#"{{"node_id":"{}","broadcast_count":{},"agents":{}}}"#,
                    state.node_id,
                    state.discovery_count,
                    state.agents.len()
                )
            })
            .unwrap_or_else(|| r#"{"error":"not_initialized"}"#.to_string())
    };
    string_to_c_str(result)
}

/// Get runtime stats
#[no_mangle]
pub extern "C" fn cortex_get_stats() -> *mut c_char {
    let result = unsafe {
        CORTEX_STATE
            .as_ref()
            .map(|s| {
                let state = s.lock().unwrap();
                let total_events: u32 = state.agents.values().map(|a| a.events_received).sum();
                format!(
                    r#"{{"node_id":"{}","agents":{},"total_events":{},"discoveries":{}}}"#,
                    state.node_id,
                    state.agents.len(),
                    total_events,
                    state.discovery_count
                )
            })
            .unwrap_or_else(|| r#"{"error":"not_initialized"}"#.to_string())
    };
    string_to_c_str(result)
}
