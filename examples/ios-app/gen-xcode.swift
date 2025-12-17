#!/usr/bin/swift
import Foundation

let fm = FileManager.default
let projectName = "CortexOS"
let baseDir = FileManager.default.currentDirectoryPath
let projectDir = "\(baseDir)/ios/\(projectName)"
let bundleId = "com.cortexos.app"

// Create directories
try fm.createDirectory(atPath: projectDir, withIntermediateDirectories: true)
try fm.createDirectory(atPath: "\(projectDir)/\(projectName)", withIntermediateDirectories: true)
try fm.createDirectory(atPath: "\(projectDir)/\(projectName).xcodeproj", withIntermediateDirectories: true)

// Write AppDelegate.swift
let appDelegateCode = """
import UIKit

@main
class AppDelegate: UIResponder, UIApplicationDelegate {
    var window: UIWindow?
    
    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        window = UIWindow(frame: UIScreen.main.bounds)
        let vc = ViewController()
        window?.rootViewController = UINavigationController(rootViewController: vc)
        window?.makeKeyAndVisible()
        
        // Init CortexOS
        let ok = cortex_init()
        print("✅ CortexOS initialized: \\(ok)")
        
        return true
    }
}
"""

try appDelegateCode.write(toFile: "\(projectDir)/\(projectName)/AppDelegate.swift", atomically: true, encoding: .utf8)

// Write ViewController.swift
let viewControllerCode = """
import UIKit

class ViewController: UIViewController {
    let statusLabel = UILabel()
    let textField = UITextField()
    let button = UIButton()
    
    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .white
        title = "CortexOS"
        
        // Status
        statusLabel.text = "Ready"
        statusLabel.textAlignment = .center
        statusLabel.font = .systemFont(ofSize: 16)
        view.addSubview(statusLabel)
        statusLabel.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            statusLabel.topAnchor.constraint(equalTo: view.safeAreaLayoutGuide.topAnchor, constant: 20),
            statusLabel.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: 16),
            statusLabel.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -16)
        ])
        
        // Text field
        textField.placeholder = "Agent name"
        textField.borderStyle = .roundedRect
        view.addSubview(textField)
        textField.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            textField.topAnchor.constraint(equalTo: statusLabel.bottomAnchor, constant: 40),
            textField.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: 16),
            textField.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -16),
            textField.heightAnchor.constraint(equalToConstant: 44)
        ])
        
        // Button
        button.setTitle("Start Agent", for: .normal)
        button.backgroundColor = .systemBlue
        button.setTitleColor(.white, for: .normal)
        button.layer.cornerRadius = 8
        view.addSubview(button)
        button.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            button.topAnchor.constraint(equalTo: textField.bottomAnchor, constant: 16),
            button.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: 16),
            button.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -16),
            button.heightAnchor.constraint(equalToConstant: 44)
        ])
        
        button.addTarget(self, action: #selector(startAgent), for: .touchUpInside)
    }
    
    @objc func startAgent() {
        guard let name = textField.text, !name.isEmpty else {
            statusLabel.text = "Enter name"
            return
        }
        
        let msg = cortex_start_agent(name: name)
        statusLabel.text = msg
        textField.text = ""
    }
}
"""

try viewControllerCode.write(toFile: "\(projectDir)/\(projectName)/ViewController.swift", atomically: true, encoding: .utf8)

// Create bridging header
let bridgingHeader = """
//
//  CortexOS-Bridging-Header.h
//
#ifndef CortexOS_Bridging_Header_h
#define CortexOS_Bridging_Header_h

void cortex_init(void);
char* cortex_start_agent(const char* name);
char* cortex_send_event(const char* agent_id, const char* payload);

#endif /* CortexOS_Bridging_Header_h */
"""

try bridgingHeader.write(toFile: "\(projectDir)/\(projectName)/CortexOS-Bridging-Header.h", atomically: true, encoding: .utf8)

print("✅ Created Swift files")
print("📁 Files created in: \(projectDir)")
