#!/bin/bash
# WASM Build Benchmark Script
# Measures build times and binary sizes for cortex-core WASM builds

set -e

echo "========================================="
echo "CortexOS WASM Build Benchmark"
echo "========================================="
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Ensure wasm32-wasip1 target is installed
echo -e "${BLUE}Checking WASM target...${NC}"
rustup target add wasm32-wasip1 2>/dev/null || echo "wasm32-wasip1 already installed"
echo ""

# Clean previous builds
echo -e "${BLUE}Cleaning previous builds...${NC}"
cargo clean -p cortex-core 2>/dev/null || true
echo ""

# Debug build
echo -e "${YELLOW}=== Debug Build ===${NC}"
echo -e "${BLUE}Building cortex-core (debug)...${NC}"
START_TIME=$(date +%s)
cargo build --target wasm32-wasip1 -p cortex-core --quiet
END_TIME=$(date +%s)
DEBUG_TIME=$((END_TIME - START_TIME))

DEBUG_RLIB=$(find target/wasm32-wasip1/debug -name "libcortex_core*.rlib" -type f | head -1)
if [ -f "$DEBUG_RLIB" ]; then
    DEBUG_SIZE=$(ls -lh "$DEBUG_RLIB" | awk '{print $5}')
    DEBUG_SIZE_BYTES=$(stat -f%z "$DEBUG_RLIB" 2>/dev/null || stat -c%s "$DEBUG_RLIB")
    echo -e "${GREEN}✓ Debug build completed${NC}"
    echo "  Time: ${DEBUG_TIME}s"
    echo "  Size: ${DEBUG_SIZE}"
else
    echo -e "${RED}✗ Debug build failed${NC}"
    exit 1
fi
echo ""

# Release build
echo -e "${YELLOW}=== Release Build ===${NC}"
echo -e "${BLUE}Building cortex-core (release)...${NC}"
START_TIME=$(date +%s)
cargo build --target wasm32-wasip1 -p cortex-core --release --quiet
END_TIME=$(date +%s)
RELEASE_TIME=$((END_TIME - START_TIME))

RELEASE_RLIB=$(find target/wasm32-wasip1/release -name "libcortex_core*.rlib" -type f | head -1)
if [ -f "$RELEASE_RLIB" ]; then
    RELEASE_SIZE=$(ls -lh "$RELEASE_RLIB" | awk '{print $5}')
    RELEASE_SIZE_BYTES=$(stat -f%z "$RELEASE_RLIB" 2>/dev/null || stat -c%s "$RELEASE_RLIB")
    echo -e "${GREEN}✓ Release build completed${NC}"
    echo "  Time: ${RELEASE_TIME}s"
    echo "  Size: ${RELEASE_SIZE}"
else
    echo -e "${RED}✗ Release build failed${NC}"
    exit 1
fi
echo ""

# Build demo binary
echo -e "${YELLOW}=== Demo Binary Build ===${NC}"
echo -e "${BLUE}Building wasm-demo (release)...${NC}"
cd examples/wasm-demo
START_TIME=$(date +%s)
cargo build --target wasm32-wasip1 --release --quiet
END_TIME=$(date +%s)
DEMO_TIME=$((END_TIME - START_TIME))
cd ../..

DEMO_WASM="target/wasm32-wasip1/release/wasm-demo.wasm"
if [ -f "$DEMO_WASM" ]; then
    DEMO_SIZE=$(ls -lh "$DEMO_WASM" | awk '{print $5}')
    DEMO_SIZE_BYTES=$(stat -f%z "$DEMO_WASM" 2>/dev/null || stat -c%s "$DEMO_WASM")
    echo -e "${GREEN}✓ Demo binary build completed${NC}"
    echo "  Time: ${DEMO_TIME}s"
    echo "  Size: ${DEMO_SIZE}"
else
    echo -e "${RED}✗ Demo binary build failed${NC}"
    exit 1
fi
echo ""

# Summary
echo "========================================="
echo -e "${YELLOW}SUMMARY${NC}"
echo "========================================="
echo ""
echo "Build Times:"
echo "  Debug:   ${DEBUG_TIME}s"
echo "  Release: ${RELEASE_TIME}s"
echo "  Demo:    ${DEMO_TIME}s"
echo ""
echo "Binary Sizes:"
echo "  cortex-core (debug):   ${DEBUG_SIZE}"
echo "  cortex-core (release): ${RELEASE_SIZE}"
echo "  wasm-demo (binary):    ${DEMO_SIZE}"
echo ""

# Check size target
TARGET_SIZE=$((1024 * 1024)) # 1MB
if [ "$RELEASE_SIZE_BYTES" -lt "$TARGET_SIZE" ]; then
    echo -e "${GREEN}✓ Release size is under 1MB target${NC}"
else
    echo -e "${YELLOW}⚠ Release size exceeds 1MB target${NC}"
fi

# Size reduction percentage
REDUCTION=$(echo "scale=2; (1 - $RELEASE_SIZE_BYTES / $DEBUG_SIZE_BYTES) * 100" | bc)
echo ""
echo "Size Reduction (debug → release): ${REDUCTION}%"
echo ""
echo "========================================="
