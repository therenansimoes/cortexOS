#!/bin/bash
set -e

PROJECT_NAME="CortexOS"
BASE="$(pwd)"
PROJECT_DIR="$BASE/ios/$PROJECT_NAME"
RUST_LIB_DIR="/Users/renansimoes/Desktop/Projetos/cortexOS/target/aarch64-apple-ios/release"

cd "$PROJECT_DIR"

# Create Xcode project using xcodebuild
xcodebuild -create-project \
    -project "$PROJECT_NAME.xcodeproj" \
    -name "$PROJECT_NAME" \
    -type "com.apple.dt.document.workspace" 2>/dev/null || true

# Create minimal project.pbxproj
mkdir -p "$PROJECT_NAME.xcodeproj"

cat > "$PROJECT_NAME.xcodeproj/project.pbxproj" << 'PBXPROJ'
// !$*UTF8*$!
{
	archiveVersion = 1;
	classes = {
	};
	objectVersion = 55;
	objects = {
		00000001 /* Sources */ = {
			isa = PBXGroup;
			children = (
				00000011 /* AppDelegate.swift */,
				00000012 /* ViewController.swift */,
				00000013 /* CortexOS-Bridging-Header.h */,
			);
			path = CortexOS;
			sourceTree = "<group>";
		};
		00000002 /* Frameworks */ = {
			isa = PBXGroup;
			children = (
			);
			name = Frameworks;
			sourceTree = "<group>";
		};
		00000011 /* AppDelegate.swift */ = {
			isa = PBXFileReference;
			lastKnownFileType = sourcecode.swift;
			path = AppDelegate.swift;
			sourceTree = "<group>";
		};
		00000012 /* ViewController.swift */ = {
			isa = PBXFileReference;
			lastKnownFileType = sourcecode.swift;
			path = ViewController.swift;
			sourceTree = "<group>";
		};
		00000013 /* CortexOS-Bridging-Header.h */ = {
			isa = PBXFileReference;
			lastKnownFileType = sourcecode.c.h;
			path = "CortexOS-Bridging-Header.h";
			sourceTree = "<group>";
		};
		00000014 /* libcortex_ios_ffi.a */ = {
			isa = PBXFileReference;
			lastKnownFileType = archive.ar;
			name = libcortex_ios_ffi.a;
			path = "/Users/renansimoes/Desktop/Projetos/cortexOS/target/aarch64-apple-ios/release/libcortex_ios_ffi.a";
			sourceTree = SOURCE_ROOT;
		};
		00000021 /* CortexOS */ = {
			isa = PBXNativeTarget;
			buildConfigurationList = 00000031 /* Build configuration list */;
			buildPhases = (
				00000022 /* Sources */,
				00000023 /* Frameworks */,
			);
			buildRules = (
			);
			dependencies = (
			);
			name = CortexOS;
			productName = CortexOS;
			productReference = 00000041 /* CortexOS.app */;
			productType = "com.apple.product-type.application";
		};
		00000022 /* Sources */ = {
			isa = PBXSourcesBuildPhase;
			buildActionMask = 2147483647;
			files = (
				00000011 /* AppDelegate.swift */,
				00000012 /* ViewController.swift */,
			);
			runOnlyForDeploymentPostprocessing = 0;
		};
		00000023 /* Frameworks */ = {
			isa = PBXFrameworksBuildPhase;
			buildActionMask = 2147483647;
			files = (
				00000014 /* libcortex_ios_ffi.a */,
			);
			runOnlyForDeploymentPostprocessing = 0;
		};
		00000031 /* Build configuration list */ = {
			isa = XCConfigurationList;
			buildConfigurations = (
				00000032 /* Debug */,
				00000033 /* Release */,
			);
			defaultConfigurationIsVisible = 0;
			defaultConfigurationName = Release;
		};
		00000032 /* Debug */ = {
			isa = XCBuildConfiguration;
			buildSettings = {
				ARCHS = arm64;
				IPHONEOS_DEPLOYMENT_TARGET = 14.0;
				LIBRARY_SEARCH_PATHS = "/Users/renansimoes/Desktop/Projetos/cortexOS/target/aarch64-apple-ios/release";
				OTHER_LDFLAGS = "-lcortex_ios_ffi";
				PRODUCT_NAME = "$(TARGET_NAME)";
				SDKROOT = iphoneos;
				SUPPORTED_PLATFORMS = iphoneos;
				SWIFT_BRIDGING_HEADER = "CortexOS/CortexOS-Bridging-Header.h";
				SWIFT_VERSION = 5.0;
				TARGETED_DEVICE_FAMILY = "1,2";
			};
			name = Debug;
		};
		00000033 /* Release */ = {
			isa = XCBuildConfiguration;
			buildSettings = {
				ARCHS = arm64;
				IPHONEOS_DEPLOYMENT_TARGET = 14.0;
				LIBRARY_SEARCH_PATHS = "/Users/renansimoes/Desktop/Projetos/cortexOS/target/aarch64-apple-ios/release";
				OTHER_LDFLAGS = "-lcortex_ios_ffi";
				PRODUCT_NAME = "$(TARGET_NAME)";
				SDKROOT = iphoneos;
				SUPPORTED_PLATFORMS = iphoneos;
				SWIFT_BRIDGING_HEADER = "CortexOS/CortexOS-Bridging-Header.h";
				SWIFT_VERSION = 5.0;
				TARGETED_DEVICE_FAMILY = "1,2";
			};
			name = Release;
		};
		00000041 /* CortexOS.app */ = {
			isa = PBXFileReference;
			explicitFileType = wrapper.application;
			includeInIndex = 0;
			path = CortexOS.app;
			sourceTree = BUILT_PRODUCTS_DIR;
		};
		00000051 /* Project object */ = {
			isa = PBXProject;
			buildConfigurationList = 00000031 /* Build configuration list */;
			compatibilityVersion = "Xcode 13.0";
			developmentRegion = en;
			hasScannedForEncodings = 0;
			knownRegions = (
				en,
				Base,
			);
			mainGroup = 00000061 /* CortexOS */;
			projectDirPath = "";
			projectRoot = "";
			targets = (
				00000021 /* CortexOS */,
			);
		};
		00000061 /* CortexOS */ = {
			isa = PBXGroup;
			children = (
				00000001 /* Sources */,
				00000002 /* Frameworks */,
			);
			sourceTree = SOURCE_ROOT;
		};
	};
	rootObject = 00000051 /* Project object */;
}
PBXPROJ

echo "✅ Xcode project created"
open "$PROJECT_DIR/$PROJECT_NAME.xcodeproj"
