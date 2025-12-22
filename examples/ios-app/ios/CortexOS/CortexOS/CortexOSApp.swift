//
//  CortexOSApp.swift
//  CortexOS
//
//  Created by Renan Simões on 17/12/2025.
//

import SwiftUI
import NaturalLanguage

// C-compatible callback function for CoreML
func coreMLCallback(input: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>? {
    guard let input = input else { return nil }
    let text = String(cString: input)
    
    // Real AI: Language Identification using Apple's NaturalLanguage framework
    let recognizer = NLLanguageRecognizer()
    recognizer.processString(text)
    
    let language = recognizer.dominantLanguage?.rawValue ?? "unknown"
    
    // Real AI: Sentiment Analysis
    let tagger = NLTagger(tagSchemes: [.sentimentScore])
    tagger.string = text
    let (sentiment, _) = tagger.tag(at: text.startIndex, unit: .paragraph, scheme: .sentimentScore)
    let score = sentiment?.rawValue ?? "N/A"
    
    let response = "CoreML Analysis: Language=\(language), Sentiment=\(score). Input: \(text)"
    
    return strdup(response)
}

@main
struct CortexOSApp: App {
    init() {
        // Register CoreML callback for real on-device AI
        cortex_register_coreml(coreMLCallback)
    }
    
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
