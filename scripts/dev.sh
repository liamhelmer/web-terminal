#!/bin/bash
# Development workflow script
# Per spec-kit/009-deployment-spec.md
#
# This script starts both backend and frontend in development mode

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 Starting web-terminal development mode...${NC}"

# Check dependencies
check_dependency() {
    if ! command -v $1 &> /dev/null; then
        echo -e "${RED}❌ $1 is not installed${NC}"
        echo -e "${YELLOW}Install with: $2${NC}"
        exit 1
    fi
}

echo -e "${BLUE}📋 Checking dependencies...${NC}"
check_dependency "cargo" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
check_dependency "pnpm" "npm install -g pnpm@8.15.0"
check_dependency "cargo-watch" "cargo install cargo-watch"

# Install frontend dependencies if needed
if [ ! -d "frontend/node_modules" ]; then
    echo -e "${BLUE}📦 Installing frontend dependencies...${NC}"
    cd frontend
    pnpm install
    cd ..
fi

# Create log directory
mkdir -p logs

# Function to cleanup on exit
cleanup() {
    echo -e "\n${YELLOW}🛑 Stopping services...${NC}"
    kill $(jobs -p) 2>/dev/null || true
    wait
    echo -e "${GREEN}✅ Development mode stopped${NC}"
    exit 0
}

trap cleanup SIGINT SIGTERM

# Start backend (cargo watch)
echo -e "${BLUE}🦀 Starting Rust backend (cargo watch)...${NC}"
cargo watch -x run 2>&1 | sed "s/^/[BACKEND] /" | tee logs/backend.log &
BACKEND_PID=$!

# Wait for backend to start
echo -e "${YELLOW}⏳ Waiting for backend to start...${NC}"
sleep 3

# Start frontend (vite dev server)
echo -e "${BLUE}📦 Starting frontend (vite)...${NC}"
cd frontend
pnpm run dev 2>&1 | sed "s/^/[FRONTEND] /" | tee ../logs/frontend.log &
FRONTEND_PID=$!
cd ..

echo -e "${GREEN}✅ Development mode started!${NC}"
echo -e "${BLUE}Backend:  http://localhost:8080${NC}"
echo -e "${BLUE}Frontend: http://localhost:3000${NC}"
echo -e "${YELLOW}Press Ctrl+C to stop${NC}"

# Wait for both processes
wait $BACKEND_PID $FRONTEND_PID