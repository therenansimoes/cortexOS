//
//  ContentView.swift
//  CortexOS
//
//  Created by Renan Simões on 17/12/2025.
//

import SwiftUI

struct ContentView: View {
    @State private var status = "Ready"
    @State private var agentName = ""
    
    var body: some View {
        VStack(spacing: 20) {
            Text("CortexOS")
                .font(.title)
            
            Text(status)
                .font(.subheadline)
                .padding()
                .background(Color(.systemGray6))
                .cornerRadius(8)
            
            TextField("Agent name", text: $agentName)
                .textFieldStyle(.roundedBorder)
            
            Button(action: startAgent) {
                Text("Start Agent")
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.blue)
                    .foregroundColor(.white)
                    .cornerRadius(8)
            }
            
            Spacer()
        }
        .padding()
        .onAppear {
            cortex_init()
            status = "CortexOS initialized"
        }
    }
    
    func startAgent() {
        guard !agentName.isEmpty else { return }
        if let msg = cortex_start_agent(agentName) {
            status = String(cString: msg)
        }
        agentName = ""
    }
}

#Preview {
    ContentView()
}
