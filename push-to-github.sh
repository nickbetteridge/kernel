#!/bin/bash
# Push Redox Hypervisor Implementation to GitHub
# Branch: claude/incomplete-request-011CV4AVVLbJ92jwdyEE8175

set -e  # Exit on error

echo "=========================================="
echo "Redox Hypervisor - GitHub Push Helper"
echo "=========================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Configuration
BRANCH_NAME="claude/incomplete-request-011CV4AVVLbJ92jwdyEE8175"
GITHUB_USER="nickbetteridge"
GITHUB_REPO="kernel"
GITHUB_URL="https://github.com/${GITHUB_USER}/${GITHUB_REPO}.git"

# Check if we're in the kernel directory
if [ ! -f "Cargo.toml" ] || [ ! -d "src/hypervisor" ]; then
    echo -e "${RED}ERROR: This script must be run from the kernel repository root.${NC}"
    echo "Expected to find Cargo.toml and src/hypervisor/"
    echo ""
    echo "Please cd to: /home/user/ockham/redox-repos/kernel"
    echo "Then run: bash /home/user/ockham/push-to-github.sh"
    exit 1
fi

echo -e "${GREEN}✓${NC} Found kernel repository"
echo ""

# Check if branch exists
if ! git rev-parse --verify "$BRANCH_NAME" >/dev/null 2>&1; then
    echo -e "${RED}ERROR: Branch '$BRANCH_NAME' not found.${NC}"
    echo "Available branches:"
    git branch -a
    exit 1
fi

echo -e "${GREEN}✓${NC} Found branch: $BRANCH_NAME"
echo ""

# Checkout the branch
echo "Checking out branch..."
git checkout "$BRANCH_NAME"
echo -e "${GREEN}✓${NC} Switched to branch: $BRANCH_NAME"
echo ""

# Show what we're about to push
echo "=========================================="
echo "Commits to push:"
echo "=========================================="
git log --oneline --graph --decorate -10
echo ""

echo "=========================================="
echo "Files modified in this branch:"
echo "=========================================="
git diff --name-status origin/master 2>/dev/null || git diff --name-status HEAD~5..HEAD
echo ""

# Check for GitHub remote
if git remote | grep -q "^github$"; then
    echo -e "${GREEN}✓${NC} GitHub remote already exists"
    GITHUB_REMOTE_URL=$(git remote get-url github)
    echo "  URL: $GITHUB_REMOTE_URL"
else
    echo -e "${YELLOW}!${NC} Adding GitHub remote..."
    git remote add github "$GITHUB_URL"
    echo -e "${GREEN}✓${NC} Added remote 'github' -> $GITHUB_URL"
fi
echo ""

# Prompt for confirmation
echo "=========================================="
echo "Ready to Push"
echo "=========================================="
echo "Will push to: $GITHUB_URL"
echo "Branch: $BRANCH_NAME"
echo ""
echo -e "${YELLOW}Note: You will be prompted for GitHub credentials.${NC}"
echo "Use your GitHub username and a Personal Access Token (not password)."
echo ""
echo "To create a token:"
echo "1. Go to https://github.com/settings/tokens"
echo "2. Click 'Generate new token' → 'Generate new token (classic)'"
echo "3. Select scopes: 'repo' (full control of private repositories)"
echo "4. Copy the token and use it as your password"
echo ""

read -p "Continue with push? (y/N) " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Push cancelled."
    exit 0
fi

# Perform the push
echo ""
echo "Pushing to GitHub..."
echo "=========================================="

if git push github "$BRANCH_NAME"; then
    echo ""
    echo "=========================================="
    echo -e "${GREEN}✓ SUCCESS!${NC}"
    echo "=========================================="
    echo ""
    echo "Branch pushed successfully!"
    echo ""
    echo "View on GitHub:"
    echo "  https://github.com/${GITHUB_USER}/${GITHUB_REPO}/tree/${BRANCH_NAME}"
    echo ""
    echo "Create a Pull Request:"
    echo "  https://github.com/${GITHUB_USER}/${GITHUB_REPO}/pull/new/${BRANCH_NAME}"
    echo ""
    echo "Next steps:"
    echo "1. Review the changes on GitHub"
    echo "2. Create a Pull Request if you want to merge to main"
    echo "3. See HYPERVISOR_FINAL_SUMMARY.md for complete documentation"
    echo ""
else
    echo ""
    echo "=========================================="
    echo -e "${RED}✗ PUSH FAILED${NC}"
    echo "=========================================="
    echo ""
    echo "Common issues:"
    echo "1. Authentication failed - make sure you're using a Personal Access Token"
    echo "2. Network issues - check your internet connection"
    echo "3. Permission denied - verify you have push access to the repository"
    echo ""
    echo "You can try again with:"
    echo "  cd /home/user/ockham/redox-repos/kernel"
    echo "  bash /home/user/ockham/push-to-github.sh"
    echo ""
    exit 1
fi
