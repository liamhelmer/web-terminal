#!/bin/bash
# Watch and rebuild script
# Per spec-kit/009-deployment-spec.md
#
# Watches for file changes and rebuilds automatically

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}👀 Starting watch mode...${NC}"

# Check if cargo-watch is installed
if ! command -v cargo-watch &> /dev/null; then
    echo -e "${RED}❌ cargo-watch is not installed${NC}"
    echo -e "${BLUE}Installing cargo-watch...${NC}"
    cargo install cargo-watch
fi

# Parse command line arguments
TARGET="${1:-all}"

case "$TARGET" in
    backend|rust)
        echo -e "${BLUE}👀 Watching Rust backend...${NC}"
        cargo watch -x check -x test -x run
        ;;

    frontend|js|ts)
        echo -e "${BLUE}👀 Watching frontend...${NC}"
        cd frontend
        pnpm run dev
        ;;

    all)
        echo -e "${BLUE}👀 Watching both backend and frontend...${NC}"
        # Use dev.sh for watching both
        ./scripts/dev.sh
        ;;

    *)
        echo -e "${RED}❌ Unknown target: $TARGET${NC}"
        echo -e "Usage: $0 [backend|frontend|all]"
        exit 1
        ;;
esac