#!/bin/bash
# Docker build script
# Per spec-kit/009-deployment-spec.md
#
# Builds Docker images with proper tagging and multi-arch support

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Default values
IMAGE_NAME="web-terminal"
REGISTRY="ghcr.io/liamhelmer"
VERSION=$(git describe --tags --always --dirty)
MULTI_ARCH=false
PUSH=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --multi-arch)
            MULTI_ARCH=true
            shift
            ;;
        --push)
            PUSH=true
            shift
            ;;
        --tag)
            VERSION="$2"
            shift 2
            ;;
        --registry)
            REGISTRY="$2"
            shift 2
            ;;
        *)
            echo -e "${RED}❌ Unknown option: $1${NC}"
            echo "Usage: $0 [--multi-arch] [--push] [--tag VERSION] [--registry REGISTRY]"
            exit 1
            ;;
    esac
done

FULL_IMAGE="${REGISTRY}/${IMAGE_NAME}"

echo -e "${BLUE}🐳 Building Docker image...${NC}"
echo -e "${BLUE}Image: ${FULL_IMAGE}:${VERSION}${NC}"
echo -e "${BLUE}Multi-arch: ${MULTI_ARCH}${NC}"
echo -e "${BLUE}Push: ${PUSH}${NC}"

# Check if docker is installed
if ! command -v docker &> /dev/null; then
    echo -e "${RED}❌ Docker is not installed${NC}"
    exit 1
fi

# Build single-arch image
if [ "$MULTI_ARCH" = false ]; then
    echo -e "${BLUE}📦 Building single-arch image...${NC}"

    docker build \
        --tag "${FULL_IMAGE}:${VERSION}" \
        --tag "${FULL_IMAGE}:latest" \
        --build-arg VERSION="${VERSION}" \
        --build-arg BUILD_DATE="$(date -u +"%Y-%m-%dT%H:%M:%SZ")" \
        --build-arg VCS_REF="$(git rev-parse --short HEAD)" \
        .

    if [ "$PUSH" = true ]; then
        echo -e "${BLUE}⬆️  Pushing image...${NC}"
        docker push "${FULL_IMAGE}:${VERSION}"
        docker push "${FULL_IMAGE}:latest"
    fi

    echo -e "${GREEN}✅ Single-arch image built successfully!${NC}"
    echo -e "${BLUE}Run with: docker run -p 8080:8080 ${FULL_IMAGE}:${VERSION}${NC}"

else
    # Build multi-arch image (requires buildx)
    echo -e "${BLUE}📦 Building multi-arch image (amd64, arm64)...${NC}"

    # Create buildx builder if it doesn't exist
    if ! docker buildx inspect multiarch &> /dev/null; then
        echo -e "${YELLOW}Creating buildx builder...${NC}"
        docker buildx create --name multiarch --use
    else
        docker buildx use multiarch
    fi

    # Ensure builder is running
    docker buildx inspect --bootstrap

    BUILD_ARGS=(
        "buildx" "build"
        "--platform" "linux/amd64,linux/arm64"
        "--tag" "${FULL_IMAGE}:${VERSION}"
        "--tag" "${FULL_IMAGE}:latest"
        "--build-arg" "VERSION=${VERSION}"
        "--build-arg" "BUILD_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
        "--build-arg" "VCS_REF=$(git rev-parse --short HEAD)"
    )

    if [ "$PUSH" = true ]; then
        BUILD_ARGS+=("--push")
    else
        BUILD_ARGS+=("--load")
    fi

    BUILD_ARGS+=(".")

    docker "${BUILD_ARGS[@]}"

    echo -e "${GREEN}✅ Multi-arch image built successfully!${NC}"
    echo -e "${BLUE}Architectures: linux/amd64, linux/arm64${NC}"

    if [ "$PUSH" = true ]; then
        echo -e "${GREEN}✅ Images pushed to ${FULL_IMAGE}${NC}"
    fi
fi

# Display image info
echo -e "\n${BLUE}📋 Image information:${NC}"
docker images "${FULL_IMAGE}" | head -2

echo -e "\n${GREEN}✅ Docker build complete!${NC}"