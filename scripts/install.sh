#!/bin/bash
# Lexum Installation Script for Linux
# Downloads and installs Lexum directly from GitHub repository

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
REPO_URL="https://github.com/hivellm/lexum.git"
INSTALL_DIR="${LEXUM_INSTALL_DIR:-/opt/lexum}"
BIN_DIR="${LEXUM_BIN_DIR:-/usr/local/bin}"
SERVICE_NAME="lexum"
SERVICE_USER="${LEXUM_USER:-lexum}"
DATA_DIR="${LEXUM_DATA_DIR:-/var/lib/lexum}"
CONFIG_DIR="${LEXUM_CONFIG_DIR:-/etc/lexum}"

# Detect architecture
ARCH=$(uname -m)
case $ARCH in
    x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
    aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
    armv7l) TARGET="armv7-unknown-linux-gnueabihf" ;;
    *) echo -e "${RED}Unsupported architecture: $ARCH${NC}" && exit 1 ;;
esac

# Detect OS
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$ID
else
    echo -e "${RED}Cannot detect OS${NC}"
    exit 1
fi

echo -e "${GREEN}🚀 Lexum Installation Script${NC}"
echo -e "${GREEN}===========================${NC}"
echo ""
echo "Repository: $REPO_URL"
echo "Install directory: $INSTALL_DIR"
echo "Binary directory: $BIN_DIR"
echo "Service name: $SERVICE_NAME"
echo "Architecture: $ARCH ($TARGET)"
echo ""

# Check if running as root for service installation
if [ "$EUID" -ne 0 ]; then 
    echo -e "${YELLOW}⚠️  Note: Some steps require sudo privileges${NC}"
    SUDO="sudo"
else
    SUDO=""
fi

# Install dependencies
echo -e "${GREEN}📦 Installing dependencies...${NC}"
case $OS in
    ubuntu|debian)
        $SUDO apt-get update
        $SUDO apt-get install -y curl git build-essential pkg-config libssl-dev ca-certificates
        ;;
    fedora|rhel|centos)
        $SUDO dnf install -y curl git gcc gcc-c++ make pkgconfig openssl-devel ca-certificates
        ;;
    arch|manjaro)
        $SUDO pacman -S --noconfirm curl git base-devel openssl ca-certificates
        ;;
    *)
        echo -e "${YELLOW}⚠️  Unknown OS. Please install: curl, git, build-essential, pkg-config, libssl-dev${NC}"
        ;;
esac

# Install Rust if not present
if ! command -v rustc &> /dev/null; then
    echo -e "${GREEN}🦀 Installing Rust...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    export PATH="$HOME/.cargo/bin:$PATH"
    source "$HOME/.cargo/env"
else
    echo -e "${GREEN}✅ Rust already installed${NC}"
    rustc --version
fi

# Create temporary directory for build
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

echo -e "${GREEN}📥 Cloning repository...${NC}"
git clone --depth 1 "$REPO_URL" "$TEMP_DIR/lexum"

cd "$TEMP_DIR/lexum"

# Build release binary
echo -e "${GREEN}🔨 Building Lexum (this may take a while)...${NC}"
export CARGO_TARGET_DIR="$TEMP_DIR/target"
cargo build --release --bin lexum-server

# Create directories
echo -e "${GREEN}📁 Creating directories...${NC}"
$SUDO mkdir -p "$INSTALL_DIR/bin"
$SUDO mkdir -p "$DATA_DIR"
$SUDO mkdir -p "$CONFIG_DIR"

# Install binary
echo -e "${GREEN}📦 Installing binary...${NC}"
$SUDO cp "$TEMP_DIR/target/release/lexum-server" "$INSTALL_DIR/bin/lexum-server"
$SUDO chmod +x "$INSTALL_DIR/bin/lexum-server"

# Create symlink for CLI
echo -e "${GREEN}🔗 Creating CLI symlink...${NC}"
$SUDO ln -sf "$INSTALL_DIR/bin/lexum-server" "$BIN_DIR/lexum"

# Create service user if it doesn't exist
if ! id "$SERVICE_USER" &>/dev/null; then
    echo -e "${GREEN}👤 Creating service user...${NC}"
    $SUDO useradd -r -s /bin/false -d "$DATA_DIR" "$SERVICE_USER"
fi

# Set ownership
$SUDO chown -R "$SERVICE_USER:$SERVICE_USER" "$INSTALL_DIR" "$DATA_DIR" "$CONFIG_DIR"

# Create systemd service file
echo -e "${GREEN}⚙️  Creating systemd service...${NC}"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"
$SUDO tee "$SERVICE_FILE" > /dev/null <<EOF
[Unit]
Description=Lexum Search Engine Server
Documentation=https://github.com/hivellm/lexum
After=network.target

[Service]
Type=simple
User=$SERVICE_USER
Group=$SERVICE_USER
WorkingDirectory=$DATA_DIR
ExecStart=$INSTALL_DIR/bin/lexum-server
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=lexum

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$DATA_DIR $CONFIG_DIR

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
EOF

# Copy example config if it exists
if [ -f "$TEMP_DIR/lexum/config.example.yml" ]; then
    if [ ! -f "$CONFIG_DIR/config.yml" ]; then
        echo -e "${GREEN}📝 Creating default configuration...${NC}"
        $SUDO cp "$TEMP_DIR/lexum/config.example.yml" "$CONFIG_DIR/config.yml"
        $SUDO chown "$SERVICE_USER:$SERVICE_USER" "$CONFIG_DIR/config.yml"
    fi
fi

# Reload systemd and enable service
echo -e "${GREEN}🔄 Enabling and starting service...${NC}"
$SUDO systemctl daemon-reload
$SUDO systemctl enable "$SERVICE_NAME"
$SUDO systemctl start "$SERVICE_NAME"

# Wait a moment for service to start
sleep 2

# Check service status
if $SUDO systemctl is-active --quiet "$SERVICE_NAME"; then
    echo -e "${GREEN}✅ Service started successfully${NC}"
else
    echo -e "${YELLOW}⚠️  Service may not have started. Check status with: sudo systemctl status $SERVICE_NAME${NC}"
fi

echo ""
echo -e "${GREEN}🎉 Lexum installation complete!${NC}"
echo ""
echo "Installation details:"
echo "  Binary: $INSTALL_DIR/bin/lexum-server"
echo "  CLI: $BIN_DIR/lexum"
echo "  Data: $DATA_DIR"
echo "  Config: $CONFIG_DIR"
echo "  Service: $SERVICE_NAME"
echo ""
echo "Useful commands:"
echo "  Check status: sudo systemctl status $SERVICE_NAME"
echo "  View logs: sudo journalctl -u $SERVICE_NAME -f"
echo "  Restart: sudo systemctl restart $SERVICE_NAME"
echo "  Stop: sudo systemctl stop $SERVICE_NAME"
echo ""
echo "Test CLI: lexum --help"
echo ""

