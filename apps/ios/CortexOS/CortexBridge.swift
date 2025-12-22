// CortexOS Swift Bridge
// Wraps the Rust FFI into Swift-friendly API

import Foundation

class CortexBridge {
    static let shared = CortexBridge()
    
    private init() {
        let initialized = cortex_init()
        print("CortexOS initialized: \(initialized)")
    }
    
    // MARK: - Core
    
    var nodeId: String {
        guard let ptr = cortex_get_node_id() else { return "" }
        let str = String(cString: ptr)
        cortex_free_string(ptr)
        return str
    }
    
    // MARK: - Agents
    
    func startInferenceAgent(name: String) -> String {
        guard let ptr = cortex_start_inference_agent(name) else { return "" }
        let result = String(cString: ptr)
        cortex_free_string(ptr)
        return result
    }
    
    func startHeartbeatAgent(name: String, interval: UInt64) -> String {
        guard let ptr = cortex_start_heartbeat_agent(name, interval) else { return "" }
        let result = String(cString: ptr)
        cortex_free_string(ptr)
        return result
    }
    
    func startLoggerAgent(name: String) -> String {
        guard let ptr = cortex_start_logger_agent(name) else { return "" }
        let result = String(cString: ptr)
        cortex_free_string(ptr)
        return result
    }
    
    func startRemoteInferenceAgent(name: String, url: String, model: String) -> String {
        guard let ptr = cortex_start_remote_inference_agent(name, url, model) else { return "" }
        let result = String(cString: ptr)
        cortex_free_string(ptr)
        return result
    }
    
    var agentCount: Int32 {
        cortex_agent_count()
    }
    
    func listAgents() -> [[String: Any]] {
        guard let ptr = cortex_list_agents() else { return [] }
        let json = String(cString: ptr)
        cortex_free_string(ptr)
        
        guard let data = json.data(using: .utf8),
              let array = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]] else {
            return []
        }
        return array
    }
    
    func stopAgent(id: String) -> Bool {
        cortex_stop_agent(id)
    }
    
    func removeAgent(id: String) -> Bool {
        cortex_remove_agent(id)
    }
    
    // MARK: - Messaging
    
    func sendToAgent(agentId: String, message: String) -> String {
        guard let ptr = cortex_send_to_agent(agentId, message) else { return "" }
        let result = String(cString: ptr)
        cortex_free_string(ptr)
        return result
    }
    
    func publishEvent(kind: String, payload: String) -> String {
        guard let ptr = cortex_publish_event(kind, payload) else { return "" }
        let result = String(cString: ptr)
        cortex_free_string(ptr)
        return result
    }
    
    // MARK: - Discovery
    
    func broadcastDiscovery() -> String {
        guard let ptr = cortex_broadcast_discovery() else { return "" }
        let result = String(cString: ptr)
        cortex_free_string(ptr)
        return result
    }
    
    // MARK: - Stats
    
    func getStats() -> [String: Any] {
        guard let ptr = cortex_get_stats() else { return [:] }
        let json = String(cString: ptr)
        cortex_free_string(ptr)
        
        guard let data = json.data(using: .utf8),
              let dict = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            return [:]
        }
        return dict
    }
    
    func getEventLog() -> [String] {
        guard let ptr = cortex_get_event_log() else { return [] }
        let json = String(cString: ptr)
        cortex_free_string(ptr)
        
        guard let data = json.data(using: .utf8),
              let array = try? JSONSerialization.jsonObject(with: data) as? [String] else {
            return []
        }
        return array
    }
}

