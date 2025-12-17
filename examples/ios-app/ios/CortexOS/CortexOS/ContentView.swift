//
//  ContentView.swift
//  CortexOS
//
//  Created by Renan Simões on 17/12/2025.
//
//  Zero Mock Policy: All UI calls real Rust implementations

import SwiftUI

// MARK: - Models (parsed from real Rust JSON)

struct Agent: Identifiable, Codable {
    let id: String
    let status: String
    var name: String = ""
    var type: String = ""
    var events: Int = 0
}

struct RuntimeStats: Codable {
    let node_id: String
    let agents: Int
    let running: Int?
    let total_events: Int?
    let discoveries: Int?
    let log_size: Int?
}

enum AgentType: String, CaseIterable {
    case inference = "Local AI"
    case remoteInference = "Remote AI"
    case coreML = "CoreML AI"
    case logger = "Logger"
    case heartbeat = "Heartbeat"
    
    var icon: String {
        switch self {
        case .inference: return "brain"
        case .remoteInference: return "cloud.fill"
        case .coreML: return "apple.logo"
        case .logger: return "doc.text.fill"
        case .heartbeat: return "heart.fill"
        }
    }
    
    var description: String {
        switch self {
        case .inference: return "Local rule-based inference (Offline)"
        case .remoteInference: return "Connects to Ollama/LlamaCpp via HTTP"
        case .coreML: return "Uses Apple CoreML/NaturalLanguage (On-Device)"
        case .logger: return "Logs all events it receives"
        case .heartbeat: return "Emits periodic heartbeat events"
        }
    }
}

// MARK: - Main View

struct ContentView: View {
    @State private var nodeId = ""
    @State private var agents: [Agent] = []
    @State private var stats = RuntimeStats(node_id: "", agents: 0, running: nil, total_events: nil, discoveries: nil, log_size: nil)
    
    // Form State
    @State private var agentName = ""
    @State private var selectedAgentType: AgentType = .inference
    @State private var heartbeatInterval: Double = 5
    @State private var remoteUrl = "http://192.168.1.100:11434"
    @State private var remoteModel = "llama3"
    
    // Chat State
    @State private var messageText = ""
    @State private var selectedAgentId: String? = nil
    @State private var logMessages: [String] = []
    
    // UI State
    @State private var isInitialized = false
    @State private var showAgentPicker = false
    @FocusState private var isAgentNameFocused: Bool
    @FocusState private var isMessageFocused: Bool
    @FocusState private var isUrlFocused: Bool
    
    var body: some View {
        NavigationView {
            ScrollView {
                VStack(spacing: 16) {
                    // Status Banner
                    if !isInitialized {
                        initErrorBanner
                    }
                    
                    // Header with Node Info
                    nodeInfoCard
                    
                    // Create Agent Section
                    createAgentSection
                    
                    // Agents List
                    agentsListSection
                    
                    // Chat with Agent
                    chatSection
                    
                    // Log Section
                    logSection
                }
                .padding()
            }
            .scrollDismissesKeyboard(.interactively)
            .navigationTitle("CortexOS")
            .toolbar {
                ToolbarItemGroup(placement: .keyboard) {
                    Spacer()
                    Button("Done") {
                        dismissKeyboard()
                    }
                }
            }
            .onAppear(perform: initialize)
        }
    }
    
    // MARK: - Init Error Banner
    var initErrorBanner: some View {
        HStack {
            Image(systemName: "exclamationmark.triangle.fill")
                .foregroundColor(.orange)
            Text("Runtime not initialized")
                .font(.subheadline)
            Spacer()
            Button("Retry") { initialize() }
                .font(.caption)
        }
        .padding()
        .background(Color.orange.opacity(0.1))
        .cornerRadius(8)
    }
    
    // MARK: - Node Info Card
    var nodeInfoCard: some View {
        VStack(spacing: 8) {
            HStack {
                Image(systemName: "cpu")
                    .font(.title2)
                    .foregroundColor(.blue)
                Text("Node: \(nodeId)")
                    .font(.headline)
                    .monospaced()
                Spacer()
                Circle()
                    .fill(isInitialized ? .green : .red)
                    .frame(width: 10, height: 10)
                Text(isInitialized ? "Online" : "Offline")
                    .font(.caption)
                    .foregroundColor(isInitialized ? .green : .red)
            }
            
            Divider()
            
            HStack(spacing: 20) {
                StatItem(icon: "person.2", value: "\(stats.agents)", label: "Agents")
                StatItem(icon: "bolt", value: "\(stats.total_events ?? 0)", label: "Events")
                StatItem(icon: "antenna.radiowaves.left.and.right", value: "\(stats.discoveries ?? 0)", label: "Discovery")
            }
            
            // Discovery button
            Button(action: broadcastDiscovery) {
                HStack {
                    Image(systemName: "wifi")
                    Text("Broadcast Discovery")
                }
                .font(.caption)
                .padding(.vertical, 6)
                .padding(.horizontal, 12)
                .background(Color.blue.opacity(0.2))
                .cornerRadius(8)
            }
        }
        .padding()
        .background(Color(.systemGray6))
        .cornerRadius(12)
    }
    
    // MARK: - Create Agent Section
    var createAgentSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Create Agent")
                .font(.headline)
            
            // Agent Type Picker
            Picker("Type", selection: $selectedAgentType) {
                ForEach(AgentType.allCases, id: \.self) { type in
                    HStack {
                        Image(systemName: type.icon)
                        Text(type.rawValue)
                    }.tag(type)
                }
            }
            .pickerStyle(.segmented)
            
            Text(selectedAgentType.description)
                .font(.caption)
                .foregroundColor(.secondary)
            
            // Agent Name
            TextField("Agent name", text: $agentName)
                .textFieldStyle(.roundedBorder)
                .focused($isAgentNameFocused)
                .submitLabel(.done)
                .onSubmit { isAgentNameFocused = false }
            
            // Remote Inference Config
            if selectedAgentType == .remoteInference {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Ollama Configuration")
                        .font(.caption)
                        .fontWeight(.bold)
                    
                    TextField("URL (e.g. http://10.0.0.5:11434)", text: $remoteUrl)
                        .textFieldStyle(.roundedBorder)
                        .keyboardType(.URL)
                        .textInputAutocapitalization(.never)
                        .focused($isUrlFocused)
                    
                    TextField("Model (e.g. llama3)", text: $remoteModel)
                        .textFieldStyle(.roundedBorder)
                        .textInputAutocapitalization(.never)
                }
                .padding(8)
                .background(Color.blue.opacity(0.05))
                .cornerRadius(8)
            }
            
            // Heartbeat interval (only for heartbeat)
            if selectedAgentType == .heartbeat {
                HStack {
                    Text("Interval: \(Int(heartbeatInterval))s")
                        .font(.caption)
                    Slider(value: $heartbeatInterval, in: 1...30, step: 1)
                }
            }
            
            Button(action: createAgent) {
                HStack {
                    Image(systemName: selectedAgentType.icon)
                    Text("Start \(selectedAgentType.rawValue)")
                }
                .frame(maxWidth: .infinity)
                .padding()
                .background(agentName.isEmpty ? Color.gray : Color.blue)
                .foregroundColor(.white)
                .cornerRadius(10)
            }
            .disabled(agentName.isEmpty)
        }
        .padding()
        .background(Color(.systemGray6))
        .cornerRadius(12)
    }
    
    // MARK: - Agents List
    var agentsListSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text("Running Agents (\(agents.count))")
                    .font(.headline)
                Spacer()
                Button(action: refreshAgents) {
                    Image(systemName: "arrow.clockwise")
                }
            }
            
            if agents.isEmpty {
                VStack(spacing: 8) {
                    Image(systemName: "cpu.fill")
                        .font(.largeTitle)
                        .foregroundColor(.secondary.opacity(0.5))
                    Text("No agents running")
                        .foregroundColor(.secondary)
                    Text("Create an Inference agent to chat with AI")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, 20)
            } else {
                ForEach(agents) { agent in
                    AgentRow(
                        agent: agent,
                        isSelected: selectedAgentId == agent.id,
                        onSelect: { selectedAgentId = agent.id },
                        onStop: { stopAgent(agent.id) },
                        onRemove: { removeAgent(agent.id) }
                    )
                }
            }
        }
        .padding()
        .background(Color(.systemGray6))
        .cornerRadius(12)
    }
    
    // MARK: - Chat Section
    var chatSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("Chat with Agent")
                    .font(.headline)
                Spacer()
                if let agentId = selectedAgentId, let agent = agents.first(where: { $0.id == agentId }) {
                    Text(agent.name)
                        .font(.caption)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 4)
                        .background(Color.blue.opacity(0.2))
                        .cornerRadius(6)
                    
                    Button(action: { exportDataset(agentId) }) {
                        Image(systemName: "square.and.arrow.up")
                            .font(.caption)
                    }
                }
            }
            
            if selectedAgentId == nil {
                Text("Select an agent above to chat")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .frame(maxWidth: .infinity)
                    .padding()
            } else {
                HStack {
                    TextField("Type a message...", text: $messageText)
                        .textFieldStyle(.roundedBorder)
                        .focused($isMessageFocused)
                        .submitLabel(.send)
                        .onSubmit { sendMessage() }
                    
                    Button(action: sendMessage) {
                        Image(systemName: "paperplane.fill")
                            .foregroundColor(.white)
                            .padding(10)
                            .background(messageText.isEmpty ? Color.gray : Color.blue)
                            .clipShape(Circle())
                    }
                    .disabled(messageText.isEmpty)
                }
                
                // Quick actions for inference agents
                if let agentId = selectedAgentId, 
                   let agent = agents.first(where: { $0.id == agentId }),
                   agent.type.contains("inference") {
                    ScrollView(.horizontal, showsIndicators: false) {
                        HStack(spacing: 8) {
                            QuickActionButton(title: "Olá", icon: "hand.wave") {
                                messageText = "Olá"
                                sendMessage()
                            }
                            QuickActionButton(title: "Ajuda", icon: "questionmark.circle") {
                                messageText = "ajuda"
                                sendMessage()
                            }
                            QuickActionButton(title: "Tempo", icon: "clock") {
                                messageText = "tempo"
                                sendMessage()
                            }
                            QuickActionButton(title: "2+2", icon: "plus.forwardslash.minus") {
                                messageText = "2+2"
                                sendMessage()
                            }
                            QuickActionButton(title: "CortexOS", icon: "cpu") {
                                messageText = "O que é CortexOS?"
                                sendMessage()
                            }
                        }
                    }
                }
            }
        }
        .padding()
        .background(Color(.systemGray6))
        .cornerRadius(12)
    }
    
    // MARK: - Log Section
    var logSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text("Event Log")
                    .font(.headline)
                Spacer()
                Button("Sync") {
                    syncEventLog()
                }
                .font(.caption)
                Button("Clear") {
                    logMessages.removeAll()
                }
                .font(.caption)
            }
            
            ScrollView {
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(logMessages.indices.reversed(), id: \.self) { i in
                        Text(logMessages[i])
                            .font(.system(.caption, design: .monospaced))
                            .foregroundColor(.secondary)
                            .textSelection(.enabled)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .frame(height: 180)
        }
        .padding()
        .background(Color(.systemGray6))
        .cornerRadius(12)
    }
    
    // MARK: - Actions
    
    func dismissKeyboard() {
        isAgentNameFocused = false
        isMessageFocused = false
        isUrlFocused = false
    }
    
    func initialize() {
        isInitialized = cortex_init()
        if isInitialized {
            log("✅ CortexOS runtime initialized")
            refreshNodeId()
            refreshAll()
        } else {
            log("❌ Failed to initialize runtime")
        }
    }
    
    func refreshNodeId() {
        if let ptr = cortex_get_node_id() {
            nodeId = String(cString: ptr)
            cortex_free_string(ptr)
        }
    }
    
    func refreshAll() {
        refreshAgents()
        refreshStats()
    }
    
    func refreshAgents() {
        if let ptr = cortex_list_agents() {
            let json = String(cString: ptr)
            cortex_free_string(ptr)
            
            if let data = json.data(using: .utf8),
               let decoded = try? JSONDecoder().decode([Agent].self, from: data) {
                agents = decoded
                // Auto-select first agent if none selected
                if selectedAgentId == nil && !agents.isEmpty {
                    selectedAgentId = agents.first?.id
                }
            }
        }
    }
    
    func refreshStats() {
        if let ptr = cortex_get_stats() {
            let json = String(cString: ptr)
            cortex_free_string(ptr)
            
            if let data = json.data(using: .utf8),
               let decoded = try? JSONDecoder().decode(RuntimeStats.self, from: data) {
                stats = decoded
            }
        }
    }
    
    func createAgent() {
        guard !agentName.isEmpty else { return }
        
        var resultPtr: UnsafeMutablePointer<CChar>?
        
        switch selectedAgentType {
        case .inference:
            resultPtr = cortex_start_inference_agent(agentName)
        case .remoteInference:
            resultPtr = cortex_start_remote_inference_agent(agentName, remoteUrl, remoteModel)
        case .coreML:
            resultPtr = cortex_spawn_coreml_agent(agentName)
        case .logger:
            resultPtr = cortex_start_logger_agent(agentName)
        case .heartbeat:
            resultPtr = cortex_start_heartbeat_agent(agentName, UInt(heartbeatInterval))
        }
        
        if let ptr = resultPtr {
            let result = String(cString: ptr)
            cortex_free_string(ptr)
            log("🚀 Started \(selectedAgentType.rawValue): \(result)")
            agentName = ""
            dismissKeyboard()
            refreshAll()
        }
    }
    
    func stopAgent(_ agentId: String) {
        if cortex_stop_agent(agentId) {
            log("⏸️ Stopped agent: \(agentId)")
        } else {
            log("❌ Failed to stop agent: \(agentId)")
        }
        refreshAll()
    }
    
    func removeAgent(_ agentId: String) {
        if cortex_remove_agent(agentId) {
            log("🗑️ Removed agent: \(agentId)")
            if selectedAgentId == agentId {
                selectedAgentId = nil
            }
        } else {
            log("❌ Failed to remove agent: \(agentId)")
        }
        refreshAll()
    }
    
    func sendMessage() {
        guard !messageText.isEmpty, let agentId = selectedAgentId else { return }
        
        let msg = messageText
        messageText = ""
        dismissKeyboard()
        
        log("📤 You: \(msg)")
        
        if let ptr = cortex_send_to_agent(agentId, msg) {
            let result = String(cString: ptr)
            cortex_free_string(ptr)
            
            // Parse response JSON
            if let data = result.data(using: .utf8),
               let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
                if let response = json["response"] as? String {
                    log(response)
                } else if let error = json["error"] as? String {
                    log("❌ \(error)")
                }
            }
            refreshStats()
        }
    }
    
    func broadcastDiscovery() {
        if let ptr = cortex_broadcast_discovery() {
            let result = String(cString: ptr)
            cortex_free_string(ptr)
            log("📡 \(result)")
            refreshStats()
        }
    }
    
    func syncEventLog() {
        if let ptr = cortex_get_event_log() {
            let json = String(cString: ptr)
            cortex_free_string(ptr)
            
            if let data = json.data(using: .utf8),
               let events = try? JSONDecoder().decode([String].self, from: data) {
                // Add events not already in log
                for event in events {
                    if !logMessages.contains(where: { $0.contains(event) }) {
                        logMessages.append(event)
                    }
                }
            }
        }
    }
    
    func exportDataset(_ agentId: String) {
        if let ptr = cortex_export_dataset(agentId) {
            let jsonl = String(cString: ptr)
            cortex_free_string(ptr)
            
            if jsonl.isEmpty {
                log("⚠️ No data to export for this agent")
            } else {
                UIPasteboard.general.string = jsonl
                log("💾 Dataset copied to clipboard (\(jsonl.count) bytes)")
            }
        }
    }
    
    func log(_ message: String) {
        let timestamp = DateFormatter.localizedString(from: Date(), dateStyle: .none, timeStyle: .medium)
        logMessages.append("[\(timestamp)] \(message)")
    }
}

// MARK: - Stat Item Component

struct StatItem: View {
    let icon: String
    let value: String
    let label: String
    
    var body: some View {
        VStack(spacing: 4) {
            Image(systemName: icon)
                .foregroundColor(.blue)
            Text(value)
                .font(.title2)
                .fontWeight(.bold)
            Text(label)
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity)
    }
}

// MARK: - Quick Action Button

struct QuickActionButton: View {
    let title: String
    let icon: String
    let action: () -> Void
    
    var body: some View {
        Button(action: action) {
            HStack(spacing: 4) {
                Image(systemName: icon)
                Text(title)
            }
            .font(.caption)
            .padding(.horizontal, 10)
            .padding(.vertical, 6)
            .background(Color.blue.opacity(0.15))
            .foregroundColor(.blue)
            .cornerRadius(12)
        }
    }
}

// MARK: - Agent Row Component

struct AgentRow: View {
    let agent: Agent
    let isSelected: Bool
    let onSelect: () -> Void
    let onStop: () -> Void
    let onRemove: () -> Void
    
    var statusColor: Color {
        switch agent.status {
        case "running": return .green
        case "stopped": return .gray
        default: return .orange
        }
    }
    
    var typeIcon: String {
        if agent.type.contains("inference") { return "brain" }
        if agent.type.contains("logger") { return "doc.text.fill" }
        if agent.type.contains("heartbeat") { return "heart.fill" }
        return "cpu"
    }
    
    var body: some View {
        HStack {
            // Selection indicator
            Image(systemName: isSelected ? "checkmark.circle.fill" : "circle")
                .foregroundColor(isSelected ? .blue : .secondary)
                .onTapGesture { onSelect() }
            
            Image(systemName: typeIcon)
                .foregroundColor(.blue)
                .frame(width: 24)
            
            VStack(alignment: .leading, spacing: 2) {
                Text(agent.name.isEmpty ? agent.id : agent.name)
                    .font(.subheadline)
                    .fontWeight(.medium)
                HStack {
                    Text(agent.id)
                        .font(.caption2)
                        .monospaced()
                        .foregroundColor(.secondary)
                    Text("• \(agent.events) events")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
            }
            
            Spacer()
            
            Text(agent.status)
                .font(.caption2)
                .fontWeight(.medium)
                .padding(.horizontal, 6)
                .padding(.vertical, 2)
                .background(statusColor.opacity(0.2))
                .foregroundColor(statusColor)
                .cornerRadius(4)
            
            if agent.status == "running" {
                Button(action: onStop) {
                    Image(systemName: "pause.circle.fill")
                        .foregroundColor(.orange.opacity(0.8))
                }
            }
            
            Button(action: onRemove) {
                Image(systemName: "trash.circle.fill")
                    .foregroundColor(.red.opacity(0.7))
            }
        }
        .padding(10)
        .background(isSelected ? Color.blue.opacity(0.1) : Color(.systemBackground))
        .cornerRadius(8)
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(isSelected ? Color.blue : Color.clear, lineWidth: 2)
        )
    }
}

#Preview {
    ContentView()
}
