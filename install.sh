#!/bin/bash
# Lexum Installation Script - One-liner version
# Usage: curl -fsSL https://raw.githubusercontent.com/hivellm/lexum/main/install.sh | bash

set -euo pipefail

# Download and execute the full installation script
INSTALL_SCRIPT_URL="https://raw.githubusercontent.com/hivellm/lexum/main/scripts/install.sh"

echo "🚀 Downloading Lexum installation script..."
curl -fsSL "$INSTALL_SCRIPT_URL" | bash

