#!/bin/bash
set -e

PROJECT_NAME="CortexOS"
PROJECT_DIR="$(pwd)/ios"
BUNDLE_ID="com.cortexos.app"
TEAM_ID="${APPLE_TEAM_ID:-}"

# Create project structure
mkdir -p "$PROJECT_DIR/$PROJECT_NAME"
mkdir -p "$PROJECT_DIR/$PROJECT_NAME/$PROJECT_NAME"
mkdir -p "$PROJECT_DIR/$PROJECT_NAME/Frameworks"

# Create Info.plist
cat > "$PROJECT_DIR/$PROJECT_NAME/$PROJECT_NAME/Info.plist" << 'EOF'
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
    <string>$(PRODUCT_NAME)</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSRequiresIPhoneOS</key>
    <true/>
    <key>UILaunchStoryboardName</key>
    <string>LaunchScreen</string>
    <key>UIMainStoryboardFile</key>
    <string>Main</string>
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

# Create AppDelegate
cat > "$PROJECT_DIR/$PROJECT_NAME/$PROJECT_NAME/AppDelegate.swift" << 'EOF'
import UIKit

@main
class AppDelegate: UIResponder, UIApplicationDelegate {
    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        // Initialize CortexOS runtime
        let initialized = cortex_init()
        print("CortexOS initialized: \(initialized)")
        return true
    }

    func applicationWillTerminate(_ application: UIApplication) {
        print("CortexOS shutting down")
    }
}
EOF

# Create ViewController
cat > "$PROJECT_DIR/$PROJECT_NAME/$PROJECT_NAME/ViewController.swift" << 'EOF'
import UIKit

class ViewController: UIViewController {
    @IBOutlet weak var statusLabel: UILabel!
    @IBOutlet weak var agentNameInput: UITextField!
    @IBOutlet weak var startButton: UIButton!
    
    override func viewDidLoad() {
        super.viewDidLoad()
        
        title = "CortexOS"
        startButton.addTarget(self, action: #selector(startAgent), for: .touchUpInside)
        
        updateStatus()
    }
    
    @objc func startAgent() {
        guard let name = agentNameInput.text, !name.isEmpty else {
            statusLabel.text = "Enter agent name"
            return
        }
        
        let result = cortex_start_agent(name: name)
        statusLabel.text = result
        agentNameInput.text = ""
    }
    
    func updateStatus() {
        let status = cortex_agent_status(agent_id: "system")
        statusLabel.text = status
    }
}
EOF

# Create Main.storyboard (minimal)
cat > "$PROJECT_DIR/$PROJECT_NAME/$PROJECT_NAME/Main.storyboard" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<document type="com.apple.InterfaceBuilder3.CocoaTouch.Storyboard.XIB" version="3.0" toolsVersion="22.0" targetRuntime="iOS.CocoaTouch" propertyAccessControl="none" useAutolayout="YES" useTraitCollections="YES" useSafeAreas="YES" colorLaunchImage="YES">
    <device type="iphone12" orientation="portrait" appearance="light"/>
    <dependencies>
        <plugIn identifier="com.apple.InterfaceBuilder.IBCocoaTouchPlugin" version="22.0"/>
        <capability name="Safe area layout guides" minVersion="9.0"/>
    </dependencies>
    <scenes>
        <scene sceneID="tne-QT-ifu">
            <objects>
                <viewController id="BYZ-38-t0r" customClass="ViewController" customModuleProvider="" sceneMemberID="viewController">
                    <view key="view" contentMode="scaleToFill" id="8bC-Xf-vdC">
                        <rect key="frame" x="0.0" y="0.0" width="390" height="844"/>
                        <autoresizingMask key="autoresizingMask" widthSizable="YES" heightSizable="YES"/>
                        <subviews>
                            <label opaque="NO" userInteractionEnabled="NO" contentMode="left" horizontalHuggingPriority="251" verticalHuggingPriority="251" fixedFrame="YES" text="Status: Ready" textAlignment="center" lineBreakMode="tailTruncation" baselineAdjustment="alignBaselines" adjustsFontSizeToFit="NO" translatesAutoresizingMaskIntoConstraints="NO" id="2Yk-5v-Hc0">
                                <rect key="frame" x="20" y="100" width="350" height="50"/>
                                <autoresizingMask key="autoresizingMask" flexibleWidth="YES"/>
                                <fontDescription key="fontDescription" type="system" pointSize="17"/>
                                <nil key="textColor"/>
                                <nil key="highlightedColor"/>
                            </label>
                            <textField opaque="NO" contentMode="scaleToFill" fixedFrame="YES" contentHorizontalAlignment="left" contentVerticalAlignment="center" borderStyle="roundedRect" placeholder="Agent name" textAlignment="natural" minimumFontSize="17" translatesAutoresizingMaskIntoConstraints="NO" id="xmN-lN-UKp">
                                <rect key="frame" x="20" y="200" width="280" height="40"/>
                                <autoresizingMask key="autoresizingMask" flexibleWidth="YES"/>
                                <fontDescription key="fontDescription" type="system" pointSize="14"/>
                                <textInputTraits key="textInputTraits"/>
                            </textField>
                            <button opaque="NO" contentMode="scaleToFill" fixedFrame="YES" contentHorizontalAlignment="center" contentVerticalAlignment="center" buttonType="system" lineBreakMode="middleTruncation" translatesAutoresizingMaskIntoConstraints="NO" id="8Nf-hG-mFe">
                                <rect key="frame" x="310" y="200" width="60" height="40"/>
                                <autoresizingMask key="autoresizingMask" flexibleWidth="YES"/>
                                <state key="normal" title="Start">
                                    <color key="titleColor" red="1" green="1" blue="1" alpha="1" colorSpace="custom" customColorSpace="sRGB"/>
                                </state>
                                <color key="backgroundColor" systemColor="systemBlueColor"/>
                            </button>
                        </subviews>
                        <viewLayoutGuide key="safeArea" id="6Tk-NE-Obj"/>
                        <color key="backgroundColor" systemColor="systemBackgroundColor"/>
                    </view>
                    <navigationItem key="navigationItem" id="jl8-ZC-Bpb"/>
                    <connections>
                        <outlet property="agentNameInput" destination="xmN-lN-UKp" id="FHj-4u-K0H"/>
                        <outlet property="startButton" destination="8Nf-hG-mFe" id="rBh-fd-mSa"/>
                        <outlet property="statusLabel" destination="2Yk-5v-Hc0" id="j5u-gG-XKE"/>
                    </connections>
                </viewController>
                <placeholder placeholderIdentifier="IBFirstResponder" id="dkx-z0-nzr" sceneMemberID="firstResponder"/>
            </objects>
            <point key="canvasLocation" x="0.0" y="0.0"/>
        </scene>
    </objects>
    <resources>
        <systemColor name="systemBackgroundColor">
            <color red="1" green="1" blue="1" alpha="1" colorSpace="custom" customColorSpace="sRGB"/>
        </systemColor>
        <systemColor name="systemBlueColor">
            <color red="0.0" green="0.47843137254901963" blue="1" alpha="1" colorSpace="custom" customColorSpace="sRGB"/>
        </systemColor>
    </resources>
</document>
EOF

echo "✅ Xcode project files created in $PROJECT_DIR"
echo ""
echo "Next:"
echo "1. Open Xcode"
echo "2. File → New → Project from Existing Source"
echo "3. Select: $PROJECT_DIR/$PROJECT_NAME"
echo "4. Or run: xcode-select --install && open $PROJECT_DIR/$PROJECT_NAME"
