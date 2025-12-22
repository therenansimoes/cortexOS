#!/bin/bash
# CortexOS - Free AI for Everyone
# One-click launcher script

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Colors
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}"
echo "  ██████╗ ██████╗ ██████╗ ████████╗███████╗██╗  ██╗ ██████╗ ███████╗"
echo " ██╔════╝██╔═══██╗██╔══██╗╚══██╔══╝██╔════╝╚██╗██╔╝██╔═══██╗██╔════╝"
echo " ██║     ██║   ██║██████╔╝   ██║   █████╗   ╚███╔╝ ██║   ██║███████╗"
echo " ██║     ██║   ██║██╔══██╗   ██║   ██╔══╝   ██╔██╗ ██║   ██║╚════██║"
echo " ╚██████╗╚██████╔╝██║  ██║   ██║   ███████╗██╔╝ ██╗╚██████╔╝███████║"
echo "  ╚═════╝ ╚═════╝ ╚═╝  ╚═╝   ╚═╝   ╚══════╝╚═╝  ╚═╝ ╚═════╝ ╚══════╝"
echo -e "${NC}"
echo -e "${CYAN}🌍 Free AI for Everyone - Community Powered${NC}"
echo ""

# Check if Ollama is running
if ! pgrep -x "ollama" > /dev/null; then
    echo -e "${YELLOW}Starting Ollama...${NC}"
    ollama serve &
    sleep 3
fi

# Auto-detect device capability and select model
echo -e "${CYAN}Detecting device capabilities...${NC}"
TOTAL_RAM_MB=$(sysctl -n hw.memsize 2>/dev/null | awk '{print int($1/1024/1024)}' || free -m 2>/dev/null | awk '/^Mem:/{print $2}' || echo "4096")

if [ "$TOTAL_RAM_MB" -lt 2048 ]; then
    MODEL="qwen2.5:0.5b"
    MODEL_SIZE="379MB"
    TIER="Weak device"
elif [ "$TOTAL_RAM_MB" -lt 8192 ]; then
    MODEL="qwen2.5:0.5b"  # Keep small for now, user can upgrade
    MODEL_SIZE="379MB"
    TIER="Medium device"
else
    MODEL="qwen2.5:0.5b"  # Keep small for demo
    MODEL_SIZE="379MB"
    TIER="Strong device"
fi

echo -e "${GREEN}📊 RAM: ${TOTAL_RAM_MB}MB → ${TIER}${NC}"
echo -e "${CYAN}🤖 Selected model: $MODEL ($MODEL_SIZE)${NC}"

if ! ollama list | grep -q "$MODEL"; then
    echo -e "${YELLOW}Downloading $MODEL model ($MODEL_SIZE)...${NC}"
    ollama pull $MODEL
else
    echo -e "${GREEN}✅ Model already downloaded${NC}"
fi

# Build if needed
if [ ! -f "$PROJECT_DIR/target/debug/cortex-webui" ] || [ ! -f "$PROJECT_DIR/target/debug/cortexd" ]; then
    echo -e "${YELLOW}Building CortexOS...${NC}"
    cd "$PROJECT_DIR"
    cargo build -p cortex-webui -p cortexd
fi

# Kill any existing processes
pkill -f cortexd 2>/dev/null
pkill -f cortex-webui 2>/dev/null
sleep 1

# Start the web UI
echo -e "${CYAN}Starting Web UI...${NC}"
cd "$PROJECT_DIR"
./target/debug/cortex-webui &
WEBUI_PID=$!
sleep 2

# Start a compute node with LLM skill
echo -e "${CYAN}Starting compute node...${NC}"
./target/debug/cortexd --port 7654 --skills "llm,ai,completion" --name "My-Node" &
NODE_PID=$!
sleep 3

echo ""
echo -e "${GREEN}✅ CortexOS is running!${NC}"
echo ""
echo -e "🌐 Open your browser: ${CYAN}http://localhost:8080${NC}"
echo ""
echo -e "Press ${YELLOW}Ctrl+C${NC} to stop"
echo ""

# Open browser (macOS)
if command -v open &> /dev/null; then
    open http://localhost:8080
fi

# Wait for user interrupt
trap "echo ''; echo 'Stopping CortexOS...'; kill $WEBUI_PID $NODE_PID 2>/dev/null; exit 0" INT

wait

