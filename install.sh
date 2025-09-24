#!/usr/bin/env bash

set -euo pipefail

# Configuration
REPO="rakan/GithubReview" # Update with your GitHub username
BINARY_NAME="revu"
INSTALL_DIR="${HOME}/.local/bin"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

echo "Installing ${BINARY_NAME}..."

# Get latest release
LATEST=$(curl -sL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [[ -z "$LATEST" ]]; then
    echo -e "${RED}Error: Could not determine latest release${NC}"
    exit 1
fi

echo "Latest version: ${LATEST}"

# Download
URL="https://github.com/${REPO}/releases/download/${LATEST}/revu-linux-amd64.tar.gz"
echo "Downloading from ${URL}..."

# Create install directory
mkdir -p "$INSTALL_DIR"

# Download and extract
curl -sL "$URL" | tar xz -C "$INSTALL_DIR"

# Make executable
chmod +x "$INSTALL_DIR/${BINARY_NAME}"

echo -e "${GREEN}✓ Installed ${BINARY_NAME} to ${INSTALL_DIR}/${NC}"

# Check if in PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo ""
    echo "Add to PATH by adding this to your shell config:"
    echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
fi