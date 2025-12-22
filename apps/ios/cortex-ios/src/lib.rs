//! CortexOS iOS FFI Library
//! 
//! Exposes Rust functionality to Swift via C FFI.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;
use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;
use tokio::sync::RwLock;

use cortex_core::DeviceCapabilities;
use cortex_grid::{NodeId, PeerStore, LanDiscovery, Discovery};

// Global state
static RUNTIME: OnceCell<Runtime> = OnceCell::new();
static STATE: OnceCell<Arc<RwLock<PeerState>>> = OnceCell::new();

struct PeerState {
    node_id: NodeId,
    capabilities: DeviceCapabilities,
    peer_store: Arc<RwLock<PeerStore>>,
    is_running: bool,
    chat_history: Vec<ChatMessage>,
}

struct ChatMessage {
    role: String,
    content: String,
}

fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime")
    })
}

fn get_state() -> &'static Arc<RwLock<PeerState>> {
    STATE.get_or_init(|| {
        let node_id = NodeId::random();
        let capabilities = DeviceCapabilities::detect();
        
        Arc::new(RwLock::new(PeerState {
            node_id,
            capabilities,
            peer_store: Arc::new(RwLock::new(PeerStore::new(std::time::Duration::from_secs(300)))),
            is_running: false,
            chat_history: Vec::new(),
        }))
    })
}

// ============ FFI Functions ============

/// Initialize the CortexOS peer
#[no_mangle]
pub extern "C" fn cortex_init() -> bool {
    let _ = get_runtime();
    let _ = get_state();
    true
}

/// Start the peer services
#[no_mangle]
pub extern "C" fn cortex_start(port: u16) -> bool {
    let rt = get_runtime();
    let state = get_state();
    
    rt.block_on(async {
        let mut s = state.write().await;
        if s.is_running {
            return true;
        }
        
        let node_id = s.node_id.clone();
        let peer_store = Arc::clone(&s.peer_store);
        
        // Start discovery
        let pubkey = [0u8; 32];
        let (mut discovery, mut discovery_rx) = LanDiscovery::new(node_id.clone(), pubkey, port);
        
        let peer_store_clone = Arc::clone(&peer_store);
        tokio::spawn(async move {
            while let Some(event) = discovery_rx.recv().await {
                let mut peer = cortex_grid::PeerInfo::new(event.peer_id.clone(), [0u8; 32]);
                peer.addresses = event.addresses;
                peer.capabilities.can_compute = true;
                peer_store_clone.write().await.insert(peer).await;
            }
        });
        
        tokio::spawn(async move {
            let _ = discovery.start().await;
        });
        
        s.is_running = true;
        true
    })
}

/// Stop the peer services
#[no_mangle]
pub extern "C" fn cortex_stop() -> bool {
    let rt = get_runtime();
    let state = get_state();
    
    rt.block_on(async {
        let mut s = state.write().await;
        s.is_running = false;
        true
    })
}

/// Get node ID as string (caller must free with cortex_free_string)
#[no_mangle]
pub extern "C" fn cortex_get_node_id() -> *mut c_char {
    let state = get_state();
    let rt = get_runtime();
    
    let id = rt.block_on(async {
        let s = state.read().await;
        s.node_id.to_string()
    });
    
    CString::new(id).unwrap().into_raw()
}

/// Get device info as JSON (caller must free with cortex_free_string)
#[no_mangle]
pub extern "C" fn cortex_get_device_info() -> *mut c_char {
    let state = get_state();
    let rt = get_runtime();
    
    let json = rt.block_on(async {
        let s = state.read().await;
        serde_json::json!({
            "cpu": s.capabilities.cpu.model,
            "cores": s.capabilities.cpu.cores,
            "ram_mb": s.capabilities.memory.total_mb,
            "score": s.capabilities.capacity_score,
            "max_layers": s.capabilities.max_layers,
        }).to_string()
    });
    
    CString::new(json).unwrap().into_raw()
}

/// Get peer count
#[no_mangle]
pub extern "C" fn cortex_get_peer_count() -> u32 {
    let state = get_state();
    let rt = get_runtime();
    
    rt.block_on(async {
        let s = state.read().await;
        let peers = s.peer_store.read().await;
        peers.list_active().await.len() as u32
    })
}

/// Get peers as JSON array (caller must free with cortex_free_string)
#[no_mangle]
pub extern "C" fn cortex_get_peers() -> *mut c_char {
    let state = get_state();
    let rt = get_runtime();
    
    let json = rt.block_on(async {
        let s = state.read().await;
        let peers = s.peer_store.read().await;
        let list = peers.list_active().await;
        
        let arr: Vec<serde_json::Value> = list.iter().map(|p| {
            serde_json::json!({
                "node_id": p.node_id.to_string(),
                "address": p.addresses.first().map(|a| a.to_string()).unwrap_or_default(),
            })
        }).collect();
        
        serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
    });
    
    CString::new(json).unwrap().into_raw()
}

/// Send AI query (caller must free result with cortex_free_string)
#[no_mangle]
pub extern "C" fn cortex_send_query(query: *const c_char) -> *mut c_char {
    let query_str = unsafe {
        if query.is_null() {
            return CString::new("Error: null query").unwrap().into_raw();
        }
        CStr::from_ptr(query).to_string_lossy().into_owned()
    };
    
    let state = get_state();
    let rt = get_runtime();
    
    let response = rt.block_on(async {
        let mut s = state.write().await;
        
        // Add user message
        s.chat_history.push(ChatMessage {
            role: "user".to_string(),
            content: query_str.clone(),
        });
        
        // Get peer count
        let peer_count = {
            let peers = s.peer_store.read().await;
            peers.list_active().await.len()
        };
        
        if peer_count == 0 {
            let response = "No peers available. Connect to the network first.".to_string();
            s.chat_history.push(ChatMessage {
                role: "assistant".to_string(),
                content: response.clone(),
            });
            return response;
        }
        
        // Simulate distributed processing
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        let response = format!(
            "🌐 [Distributed Response from {} peers]\n\nProcessing: \"{}\"\n\nThis response was computed across the CortexOS swarm.",
            peer_count, query_str
        );
        
        s.chat_history.push(ChatMessage {
            role: "assistant".to_string(),
            content: response.clone(),
        });
        
        response
    });
    
    CString::new(response).unwrap().into_raw()
}

/// Get chat history as JSON (caller must free with cortex_free_string)
#[no_mangle]
pub extern "C" fn cortex_get_chat_history() -> *mut c_char {
    let state = get_state();
    let rt = get_runtime();
    
    let json = rt.block_on(async {
        let s = state.read().await;
        
        let arr: Vec<serde_json::Value> = s.chat_history.iter().map(|m| {
            serde_json::json!({
                "role": m.role,
                "content": m.content,
            })
        }).collect();
        
        serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
    });
    
    CString::new(json).unwrap().into_raw()
}

/// Check if peer is running
#[no_mangle]
pub extern "C" fn cortex_is_running() -> bool {
    let state = get_state();
    let rt = get_runtime();
    
    rt.block_on(async {
        let s = state.read().await;
        s.is_running
    })
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

