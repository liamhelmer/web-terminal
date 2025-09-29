#!/bin/bash
# GitHub Repository Secrets Setup Script
# Per spec-kit/009-deployment-spec.md
#
# This script configures repository-level GitHub Actions secrets

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}🔐 Setting up GitHub Repository Secrets...${NC}"

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

# Function to generate random secret
generate_secret() {
    openssl rand -base64 32 | tr -d '\n'
}

# Generate JWT secret
echo -e "\n${BLUE}🔑 Generating JWT Secret...${NC}"
JWT_SECRET=$(generate_secret)
echo -e "${GREEN}Generated JWT secret${NC}"

# Set repository secret
echo -e "\n${BLUE}Setting repository secret: WEB_TERMINAL_JWT_SECRET${NC}"
echo "$JWT_SECRET" | gh secret set WEB_TERMINAL_JWT_SECRET

echo -e "\n${GREEN}✅ Repository secret configured!${NC}"

# Save to local .env file
if [ ! -f .env ]; then
    echo -e "\n${BLUE}💾 Creating .env file...${NC}"
    cp .env.example .env
    sed -i.bak "s/WEB_TERMINAL_JWT_SECRET=.*/WEB_TERMINAL_JWT_SECRET=${JWT_SECRET}/" .env
    rm .env.bak 2>/dev/null || true
    echo -e "${GREEN}✅ Created .env file${NC}"
else
    echo -e "${YELLOW}⚠️  .env file already exists, not overwriting${NC}"
fi

# Display configured secrets
echo -e "\n${BLUE}📋 Current repository secrets:${NC}"
gh secret list

echo -e "\n${BLUE}📋 Manual Steps Required:${NC}"
echo -e "\n${YELLOW}1. Create GitHub Environments:${NC}"
echo -e "   https://github.com/${REPO}/settings/environments"
echo -e "   - Create 'staging' environment"
echo -e "   - Create 'production' environment (with required reviewers)"

echo -e "\n${YELLOW}2. Add Environment Secrets (after creating environments):${NC}"
echo -e "   For staging environment:"
echo -e "   - STAGING_URL: http://staging.example.com:8080"
echo -e "   - KUBECONFIG_STAGING: <base64 encoded kubeconfig> (if using Kubernetes)"

echo -e "\n   For production environment:"
echo -e "   - PRODUCTION_URL: https://web-terminal.example.com"
echo -e "   - KUBECONFIG_PRODUCTION: <base64 encoded kubeconfig> (if using Kubernetes)"

echo -e "\n${YELLOW}3. Configure Branch Protection:${NC}"
echo -e "   https://github.com/${REPO}/settings/branches"
echo -e "   - Protect 'main' branch"
echo -e "   - Require status checks to pass (all CI workflows)"
echo -e "   - Require pull request reviews"

echo -e "\n${BLUE}🔐 Security Notes:${NC}"
echo -e "- JWT secret is stored in GitHub Secrets and .env"
echo -e "- Never commit .env to git (it's in .gitignore)"
echo -e "- Rotate secrets every 90 days"
echo -e "- Use OIDC for cloud deployments (no long-lived credentials)"