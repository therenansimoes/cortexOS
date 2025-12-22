// CortexOS iOS App
// Distributed AI on your iPhone

import SwiftUI

struct ContentView: View {
    @State private var nodeId: String = ""
    @State private var peerCount: Int = 0
    @State private var chatInput: String = ""
    @State private var chatMessages: [ChatMessage] = []
    @State private var isProcessing: Bool = false
    @State private var inferenceAgentId: String = ""
    @State private var showSettings: Bool = false
    @State private var isContributing: Bool = true
    
    let cortex = CortexBridge.shared
    
    var body: some View {
        NavigationView {
            VStack(spacing: 0) {
                // Header
                headerView
                
                // Main Content
                TabView {
                    chatTab
                        .tabItem {
                            Image(systemName: "brain.head.profile")
                            Text("AI Chat")
                        }
                    
                    networkTab
                        .tabItem {
                            Image(systemName: "network")
                            Text("Network")
                        }
                    
                    settingsTab
                        .tabItem {
                            Image(systemName: "gear")
                            Text("Settings")
                        }
                }
            }
            .navigationBarHidden(true)
        }
        .onAppear {
            initialize()
        }
    }
    
    // MARK: - Header
    
    var headerView: some View {
        HStack {
            VStack(alignment: .leading) {
                Text("🧠 CortexOS")
                    .font(.headline)
                Text("Node: \(String(nodeId.prefix(8)))...")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            Spacer()
            
            Circle()
                .fill(isContributing ? Color.green : Color.red)
                .frame(width: 10, height: 10)
            Text(isContributing ? "Active" : "Inactive")
                .font(.caption)
        }
        .padding()
        .background(Color(.systemBackground))
    }
    
    // MARK: - Chat Tab
    
    var chatTab: some View {
        VStack {
            // Chat Messages
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 10) {
                        ForEach(chatMessages) { message in
                            ChatBubble(message: message)
                                .id(message.id)
                        }
                    }
                    .padding()
                }
                .onChange(of: chatMessages.count) { _ in
                    if let last = chatMessages.last {
                        withAnimation {
                            proxy.scrollTo(last.id, anchor: .bottom)
                        }
                    }
                }
            }
            
            // Input
            HStack {
                TextField("Ask the AI swarm...", text: $chatInput)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                    .disabled(isProcessing)
                
                Button(action: sendMessage) {
                    if isProcessing {
                        ProgressView()
                            .frame(width: 24, height: 24)
                    } else {
                        Image(systemName: "arrow.up.circle.fill")
                            .font(.title2)
                    }
                }
                .disabled(chatInput.isEmpty || isProcessing)
            }
            .padding()
        }
    }
    
    // MARK: - Network Tab
    
    var networkTab: some View {
        VStack(spacing: 20) {
            // Stats
            VStack(spacing: 15) {
                StatRow(icon: "network", label: "Peers", value: "\(peerCount)")
                StatRow(icon: "brain", label: "Agents", value: "\(cortex.agentCount)")
                StatRow(icon: "cpu", label: "Score", value: "73/100")
            }
            .padding()
            .background(Color(.secondarySystemBackground))
            .cornerRadius(12)
            .padding(.horizontal)
            
            // Discovery
            Button(action: {
                let _ = cortex.broadcastDiscovery()
            }) {
                HStack {
                    Image(systemName: "antenna.radiowaves.left.and.right")
                    Text("Broadcast Discovery")
                }
                .frame(maxWidth: .infinity)
                .padding()
                .background(Color.blue)
                .foregroundColor(.white)
                .cornerRadius(10)
            }
            .padding(.horizontal)
            
            Spacer()
            
            Text("Device: \(UIDevice.current.name)")
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding(.top)
    }
    
    // MARK: - Settings Tab
    
    var settingsTab: some View {
        Form {
            Section("Compute") {
                Toggle("Contribute to Swarm", isOn: $isContributing)
                
                HStack {
                    Text("Max CPU")
                    Spacer()
                    Text("80%")
                        .foregroundColor(.secondary)
                }
            }
            
            Section("Network") {
                HStack {
                    Text("Port")
                    Spacer()
                    Text("7654")
                        .foregroundColor(.secondary)
                }
                
                Toggle("Open to Internet", isOn: .constant(false))
            }
            
            Section("About") {
                HStack {
                    Text("Node ID")
                    Spacer()
                    Text(nodeId)
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                }
                
                HStack {
                    Text("Version")
                    Spacer()
                    Text("0.1.0")
                        .foregroundColor(.secondary)
                }
            }
        }
    }
    
    // MARK: - Functions
    
    func initialize() {
        nodeId = cortex.nodeId
        
        // Create inference agent for chat
        let result = cortex.startInferenceAgent(name: "AI")
        if let data = result.data(using: .utf8),
           let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
           let id = json["id"] as? String {
            inferenceAgentId = id
        }
        
        // Add welcome message
        chatMessages.append(ChatMessage(
            role: "system",
            content: "🧠 CortexOS initialized!\nNode: \(String(nodeId.prefix(12)))...\n\nAsk me anything!"
        ))
        
        // Start periodic refresh
        Timer.scheduledTimer(withTimeInterval: 5, repeats: true) { _ in
            refreshStats()
        }
    }
    
    func refreshStats() {
        let stats = cortex.getStats()
        // Update peer count, etc.
    }
    
    func sendMessage() {
        guard !chatInput.isEmpty, !inferenceAgentId.isEmpty else { return }
        
        let userMessage = chatInput
        chatInput = ""
        
        // Add user message
        chatMessages.append(ChatMessage(role: "user", content: userMessage))
        
        isProcessing = true
        
        // Send to agent
        DispatchQueue.global().async {
            let response = cortex.sendToAgent(agentId: inferenceAgentId, message: userMessage)
            
            DispatchQueue.main.async {
                isProcessing = false
                
                // Parse response
                if let data = response.data(using: .utf8),
                   let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                   let content = json["response"] as? String {
                    chatMessages.append(ChatMessage(role: "assistant", content: content))
                } else {
                    chatMessages.append(ChatMessage(role: "assistant", content: "🌐 Processing across the swarm..."))
                }
            }
        }
    }
}

// MARK: - Supporting Views

struct ChatMessage: Identifiable {
    let id = UUID()
    let role: String
    let content: String
}

struct ChatBubble: View {
    let message: ChatMessage
    
    var body: some View {
        HStack {
            if message.role == "user" {
                Spacer()
            }
            
            VStack(alignment: message.role == "user" ? .trailing : .leading) {
                Text(message.role == "user" ? "You" : "🧠 AI")
                    .font(.caption)
                    .foregroundColor(.secondary)
                
                Text(message.content)
                    .padding(10)
                    .background(message.role == "user" ? Color.blue : Color(.secondarySystemBackground))
                    .foregroundColor(message.role == "user" ? .white : .primary)
                    .cornerRadius(12)
            }
            
            if message.role != "user" {
                Spacer()
            }
        }
    }
}

struct StatRow: View {
    let icon: String
    let label: String
    let value: String
    
    var body: some View {
        HStack {
            Image(systemName: icon)
                .foregroundColor(.blue)
                .frame(width: 30)
            Text(label)
            Spacer()
            Text(value)
                .fontWeight(.semibold)
        }
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView()
    }
}

