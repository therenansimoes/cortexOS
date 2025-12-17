#!/bin/bash
set -e

mkdir -p ios/CortexOS
cd ios

# Create from template using Xcode command line
xcode-select --install 2>/dev/null || true

# Create project structure manually - the correct way
PROJECT_NAME="CortexOS"
mkdir -p "$PROJECT_NAME/$PROJECT_NAME"

# AppDelegate.swift
cat > "$PROJECT_NAME/$PROJECT_NAME/AppDelegate.swift" << 'EOF'
import UIKit

@main
class AppDelegate: UIResponder, UIApplicationDelegate {
    var window: UIWindow?
    
    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        window = UIWindow(frame: UIScreen.main.bounds)
        let vc = ViewController()
        window?.rootViewController = UINavigationController(rootViewController: vc)
        window?.makeKeyAndVisible()
        
        cortex_init()
        return true
    }
}
EOF

# ViewController.swift
cat > "$PROJECT_NAME/$PROJECT_NAME/ViewController.swift" << 'EOF'
import UIKit

class ViewController: UIViewController {
    let label = UILabel()
    let textField = UITextField()
    let button = UIButton()
    
    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .white
        title = "CortexOS"
        navigationController?.isNavigationBarHidden = false
        
        label.text = "Ready"
        label.textAlignment = .center
        view.addSubview(label)
        label.frame = CGRect(x: 20, y: 100, width: view.bounds.width - 40, height: 50)
        
        textField.placeholder = "Agent name"
        textField.borderStyle = .roundedRect
        view.addSubview(textField)
        textField.frame = CGRect(x: 20, y: 200, width: view.bounds.width - 40, height: 44)
        
        button.setTitle("Start", for: .normal)
        button.backgroundColor = .systemBlue
        button.setTitleColor(.white, for: .normal)
        view.addSubview(button)
        button.frame = CGRect(x: 20, y: 270, width: view.bounds.width - 40, height: 44)
        button.addTarget(self, action: #selector(tap), for: .touchUpInside)
    }
    
    @objc func tap() {
        let name = textField.text ?? ""
        label.text = cortex_start_agent(name: name)
        textField.text = ""
    }
}
EOF

# Info.plist
cat > "$PROJECT_NAME/$PROJECT_NAME/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleDevelopmentRegion</key>
	<string>en</string>
	<key>CFBundleExecutable</key>
	<string>$(EXECUTABLE_NAME)</string>
	<key>CFBundleIdentifier</key>
	<string>$(PRODUCT_BUNDLE_IDENTIFIER)</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>CFBundleName</key>
	<string>CortexOS</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleShortVersionString</key>
	<string>1.0</string>
	<key>CFBundleVersion</key>
	<string>1</string>
	<key>LSRequiresIPhoneOS</key>
	<true/>
	<key>UIMainStoryboardFile</key>
	<string></string>
	<key>UIRequiredDeviceCapabilities</key>
	<array>
		<string>armv7</string>
	</array>
	<key>UISupportedInterfaceOrientations</key>
	<array>
		<string>UIInterfaceOrientationPortrait</string>
	</array>
</dict>
</plist>
EOF

# Bridging header
cat > "$PROJECT_NAME/$PROJECT_NAME/CortexOS-Bridging-Header.h" << 'EOF'
#ifndef CortexOS_Bridging_Header_h
#define CortexOS_Bridging_Header_h

void cortex_init(void);
char* cortex_start_agent(const char* name);

#endif
EOF

echo "✅ Files created"
