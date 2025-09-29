#!/bin/bash
# Deployment script
# Per spec-kit/009-deployment-spec.md
#
# Deploys web-terminal to specified environment

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Default values
ENVIRONMENT="${1:-staging}"
VERSION="${2:-latest}"
IMAGE_NAME="web-terminal"
REGISTRY="ghcr.io/liamhelmer"

echo -e "${BLUE}🚀 Deploying web-terminal...${NC}"
echo -e "${BLUE}Environment: ${ENVIRONMENT}${NC}"
echo -e "${BLUE}Version: ${VERSION}${NC}"

# Validate environment
case "$ENVIRONMENT" in
    staging|production)
        ;;
    *)
        echo -e "${RED}❌ Invalid environment: ${ENVIRONMENT}${NC}"
        echo "Usage: $0 [staging|production] [version]"
        exit 1
        ;;
esac

# Check if kubectl is installed
if ! command -v kubectl &> /dev/null; then
    echo -e "${RED}❌ kubectl is not installed${NC}"
    exit 1
fi

# Deployment functions
deploy_kubernetes() {
    echo -e "${BLUE}📦 Deploying to Kubernetes...${NC}"

    FULL_IMAGE="${REGISTRY}/${IMAGE_NAME}:${VERSION}"
    NAMESPACE="${ENVIRONMENT}"

    # Update deployment image
    kubectl set image deployment/web-terminal \
        web-terminal="${FULL_IMAGE}" \
        -n "${NAMESPACE}"

    # Wait for rollout
    echo -e "${YELLOW}⏳ Waiting for rollout to complete...${NC}"
    kubectl rollout status deployment/web-terminal \
        -n "${NAMESPACE}" \
        --timeout=5m

    # Verify deployment
    echo -e "${BLUE}🔍 Verifying deployment...${NC}"
    REPLICAS=$(kubectl get deployment web-terminal -n "${NAMESPACE}" -o jsonpath='{.status.availableReplicas}')

    if [ "$REPLICAS" -gt 0 ]; then
        echo -e "${GREEN}✅ Deployment successful! ${REPLICAS} replicas running${NC}"
    else
        echo -e "${RED}❌ Deployment failed: No replicas available${NC}"
        exit 1
    fi
}

deploy_docker_compose() {
    echo -e "${BLUE}📦 Deploying with Docker Compose...${NC}"

    export IMAGE_TAG="${VERSION}"
    export ENVIRONMENT="${ENVIRONMENT}"

    # Pull latest image
    docker-compose pull

    # Restart services
    docker-compose up -d

    echo -e "${GREEN}✅ Docker Compose deployment complete!${NC}"
}

# Run smoke tests
smoke_tests() {
    echo -e "${BLUE}🧪 Running smoke tests...${NC}"

    # Determine URL based on environment
    if [ "$ENVIRONMENT" = "staging" ]; then
        BASE_URL="http://staging.example.com:8080"
    else
        BASE_URL="https://web-terminal.example.com"
    fi

    # Test health endpoint
    echo -e "${YELLOW}Testing health endpoint...${NC}"
    if curl -f "${BASE_URL}/health" > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Health check passed${NC}"
    else
        echo -e "${RED}❌ Health check failed${NC}"
        return 1
    fi

    # Test main page
    echo -e "${YELLOW}Testing main page...${NC}"
    if curl -f "${BASE_URL}/" > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Main page accessible${NC}"
    else
        echo -e "${RED}❌ Main page not accessible${NC}"
        return 1
    fi

    echo -e "${GREEN}✅ All smoke tests passed!${NC}"
}

# Rollback function
rollback() {
    echo -e "${YELLOW}⚠️  Rolling back deployment...${NC}"

    kubectl rollout undo deployment/web-terminal \
        -n "${ENVIRONMENT}"

    kubectl rollout status deployment/web-terminal \
        -n "${ENVIRONMENT}" \
        --timeout=3m

    echo -e "${GREEN}✅ Rollback complete${NC}"
}

# Main deployment process
main() {
    # Ask for confirmation in production
    if [ "$ENVIRONMENT" = "production" ]; then
        echo -e "${YELLOW}⚠️  You are deploying to PRODUCTION${NC}"
        read -p "Are you sure? (yes/no): " -r
        if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
            echo -e "${RED}Deployment cancelled${NC}"
            exit 1
        fi
    fi

    # Choose deployment method based on configuration
    if kubectl cluster-info &> /dev/null; then
        deploy_kubernetes
    else
        deploy_docker_compose
    fi

    # Run smoke tests
    if ! smoke_tests; then
        echo -e "${RED}❌ Smoke tests failed! Rolling back...${NC}"
        rollback
        exit 1
    fi

    echo -e "${GREEN}🎉 Deployment complete!${NC}"
    echo -e "${BLUE}Environment: ${ENVIRONMENT}${NC}"
    echo -e "${BLUE}Version: ${VERSION}${NC}"
}

# Trap errors and rollback
trap 'echo -e "${RED}❌ Deployment failed!${NC}"; rollback; exit 1' ERR

main