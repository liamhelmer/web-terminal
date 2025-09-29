#!/bin/bash
# GitHub Secrets and Environments Setup Script
# Per spec-kit/009-deployment-spec.md
#
# This script configures GitHub Actions environments and secrets

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}🔐 Setting up GitHub Environments and Secrets...${NC}"

# Check if gh CLI is installed
if ! command -v gh &> /dev/null; then
    echo -e "${RED}❌ GitHub CLI (gh) is not installed${NC}"
    echo -e "${YELLOW}Install with: brew install gh${NC}"
    exit 1
fi

# Check if authenticated
if ! gh auth status &> /dev/null; then
    echo -e "${YELLOW}⚠️  Not authenticated with GitHub${NC}"
    echo -e "${BLUE}Running: gh auth login${NC}"
    gh auth login
fi

# Get repository name
REPO=$(gh repo view --json nameWithOwner -q .nameWithOwner)
echo -e "${BLUE}Repository: ${REPO}${NC}"

# Function to create environment
create_environment() {
    local ENV_NAME=$1
    local REQUIRE_REVIEWERS=$2

    echo -e "${BLUE}📦 Creating environment: ${ENV_NAME}${NC}"

    # Note: gh CLI doesn't support environment creation directly
    # We'll provide instructions for manual setup
    echo -e "${YELLOW}⚠️  GitHub CLI doesn't support environment creation via API${NC}"
    echo -e "${YELLOW}Please create environments manually:${NC}"
    echo -e "  1. Go to: https://github.com/${REPO}/settings/environments"
    echo -e "  2. Click 'New environment'"
    echo -e "  3. Create '${ENV_NAME}' environment"
    if [ "$REQUIRE_REVIEWERS" = true ]; then
        echo -e "  4. Enable 'Required reviewers' (for production)"
    fi
}

# Function to set secret
set_secret() {
    local ENV_NAME=$1
    local SECRET_NAME=$2
    local SECRET_VALUE=$3

    if [ -z "$SECRET_VALUE" ]; then
        echo -e "${YELLOW}⚠️  Skipping ${SECRET_NAME} (no value provided)${NC}"
        return
    fi

    if [ "$ENV_NAME" = "repo" ]; then
        echo -e "${BLUE}Setting repository secret: ${SECRET_NAME}${NC}"
        echo "$SECRET_VALUE" | gh secret set "$SECRET_NAME"
    else
        echo -e "${BLUE}Setting environment secret: ${SECRET_NAME} (${ENV_NAME})${NC}"
        echo "$SECRET_VALUE" | gh secret set "$SECRET_NAME" --env "$ENV_NAME"
    fi
}

# Function to generate random secret
generate_secret() {
    openssl rand -base64 32 | tr -d '\n'
}

echo -e "\n${BLUE}📋 Step 1: Create GitHub Environments${NC}"
create_environment "staging" false
create_environment "production" true

echo -e "\n${BLUE}Press Enter when you've created the environments...${NC}"
read -r

echo -e "\n${BLUE}🔑 Step 2: Configure Secrets${NC}"

# Generate JWT secret if not provided
JWT_SECRET="${WEB_TERMINAL_JWT_SECRET:-$(generate_secret)}"
echo -e "${GREEN}Generated JWT secret: ${JWT_SECRET:0:10}...${NC}"

# Repository-level secrets (used by all workflows)
echo -e "\n${BLUE}Setting repository-level secrets...${NC}"
set_secret "repo" "WEB_TERMINAL_JWT_SECRET" "$JWT_SECRET"

# Staging environment secrets
echo -e "\n${BLUE}Setting staging environment secrets...${NC}"
set_secret "staging" "KUBECONFIG_STAGING" "${KUBECONFIG_STAGING:-}"
set_secret "staging" "STAGING_URL" "${STAGING_URL:-http://staging.example.com:8080}"

# Production environment secrets
echo -e "\n${BLUE}Setting production environment secrets...${NC}"
set_secret "production" "KUBECONFIG_PRODUCTION" "${KUBECONFIG_PRODUCTION:-}"
set_secret "production" "PRODUCTION_URL" "${PRODUCTION_URL:-https://web-terminal.example.com}"

# Optional: Docker registry credentials
if [ -n "$DOCKER_USERNAME" ]; then
    echo -e "\n${BLUE}Setting Docker registry credentials...${NC}"
    set_secret "repo" "DOCKER_USERNAME" "$DOCKER_USERNAME"
    set_secret "repo" "DOCKER_PASSWORD" "$DOCKER_PASSWORD"
fi

# Optional: Cloud provider credentials (AWS, Azure, GCP)
if [ -n "$AWS_ROLE_ARN" ]; then
    echo -e "\n${BLUE}Setting AWS OIDC credentials...${NC}"
    set_secret "production" "AWS_ROLE_ARN" "$AWS_ROLE_ARN"
    set_secret "production" "AWS_REGION" "${AWS_REGION:-us-east-1}"
fi

echo -e "\n${GREEN}✅ GitHub secrets configuration complete!${NC}"

# Display summary
echo -e "\n${BLUE}📋 Configuration Summary:${NC}"
echo -e "${GREEN}✅ Repository secrets:${NC}"
gh secret list

echo -e "\n${BLUE}📋 Next Steps:${NC}"
echo -e "1. Verify environments are created: https://github.com/${REPO}/settings/environments"
echo -e "2. Add required reviewers to production environment"
echo -e "3. Configure branch protection rules for main branch"
echo -e "4. Test GitHub Actions workflows with a test commit"

# Save JWT secret to local .env file
if [ ! -f .env ]; then
    echo -e "\n${BLUE}💾 Saving secrets to .env file...${NC}"
    cp .env.example .env
    sed -i.bak "s/WEB_TERMINAL_JWT_SECRET=.*/WEB_TERMINAL_JWT_SECRET=${JWT_SECRET}/" .env
    rm .env.bak 2>/dev/null || true
    echo -e "${GREEN}✅ Created .env file with JWT secret${NC}"
fi

echo -e "\n${BLUE}🔐 Important Security Notes:${NC}"
echo -e "- Never commit .env file to git"
echo -e "- Rotate JWT secrets regularly (every 90 days)"
echo -e "- Use OIDC for cloud deployments (no long-lived secrets)"
echo -e "- Enable branch protection on main branch"
echo -e "- Require approval for production deployments"