#!/usr/bin/env bash
set -e

# apidrift installer script
# Usage: curl -sSfL https://raw.githubusercontent.com/sensiarion/apidrift/main/install.sh | bash

VERSION="${APIDRIFT_VERSION:-latest}"
INSTALL_DIR="${APIDRIFT_INSTALL_DIR:-/usr/local/bin}"
REPO_URL="${APIDRIFT_REPO_URL:-https://github.com/sensiarion/apidrift}"
GITHUB_API_URL="https://api.github.com/repos/sensiarion/apidrift"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Detect OS and architecture
detect_platform() {
    local os=""
    local arch=""
    
    # Detect OS
    case "$(uname -s)" in
        Linux*)     os="linux";;
        Darwin*)    os="macos";;
        CYGWIN*|MINGW*|MSYS*) os="windows";;
        *)          log_error "Unsupported OS: $(uname -s)"; exit 1;;
    esac
    
    # Detect architecture
    case "$(uname -m)" in
        x86_64|amd64)   arch="x86_64";;
        aarch64|arm64)  arch="arm64";;
        *)              log_error "Unsupported architecture: $(uname -m)"; exit 1;;
    esac
    
    echo "${os}-${arch}"
}

# Check if running as root or with sudo
check_install_permissions() {
    if [ ! -w "$INSTALL_DIR" ]; then
        log_warn "Installation directory $INSTALL_DIR is not writable."
        log_warn "You may need to run this script with sudo or choose a different directory."
        
        # Suggest user local bin
        if [ "$INSTALL_DIR" = "/usr/local/bin" ]; then
            log_info "Consider setting APIDRIFT_INSTALL_DIR to ~/.local/bin"
            log_info "Example: APIDRIFT_INSTALL_DIR=~/.local/bin curl -sSfL ... | bash"
            
            read -p "Install to ~/.local/bin instead? (y/N) " -n 1 -r
            echo
            if [[ $REPLY =~ ^[Yy]$ ]]; then
                INSTALL_DIR="$HOME/.local/bin"
                mkdir -p "$INSTALL_DIR"
            else
                log_error "Installation cancelled."
                exit 1
            fi
        else
            exit 1
        fi
    fi
}

# Download and install binary
install_apidrift() {
    local platform=$1
    local version=$2
    local temp_dir=$(mktemp -d)
    
    log_info "Installing apidrift for platform: $platform"
    log_info "Version: $version"
    log_info "Install directory: $INSTALL_DIR"
    
    # Construct download URL
    local filename=""
    local extension=""
    
    if [[ "$platform" == *"windows"* ]]; then
        extension="zip"
    else
        extension="tar.gz"
    fi
    
    filename="apidrift-${platform}.${extension}"
    
    # For latest version, fetch the latest release from GitHub
    # For specific versions, use tagged releases
    local download_url=""
    if [ "$version" = "latest" ]; then
        log_info "Fetching latest release information..."
        
        # Get the latest release tag from GitHub API
        local latest_tag=""
        if command -v curl &> /dev/null; then
            latest_tag=$(curl -sSfL "${GITHUB_API_URL}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/' || echo "")
        elif command -v wget &> /dev/null; then
            latest_tag=$(wget -qO- "${GITHUB_API_URL}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/' || echo "")
        fi
        
        if [ -z "$latest_tag" ]; then
            log_error "Failed to fetch latest release information"
            log_info "Please specify a version manually: APIDRIFT_VERSION=v0.1.0"
            rm -rf "$temp_dir"
            exit 1
        fi
        
        version="$latest_tag"
        log_info "Latest version: $version"
    fi
    
    # Construct GitHub release download URL
    download_url="${REPO_URL}/releases/download/${version}/${filename}"
    
    log_info "Downloading from: $download_url"
    
    # Download the archive
    if command -v curl &> /dev/null; then
        curl -sSfL "$download_url" -o "$temp_dir/$filename" || {
            log_error "Failed to download apidrift"
            log_error "URL: $download_url"
            rm -rf "$temp_dir"
            exit 1
        }
    elif command -v wget &> /dev/null; then
        wget -q "$download_url" -O "$temp_dir/$filename" || {
            log_error "Failed to download apidrift"
            rm -rf "$temp_dir"
            exit 1
        }
    else
        log_error "Neither curl nor wget is available. Please install one of them."
        rm -rf "$temp_dir"
        exit 1
    fi
    
    # Extract the archive
    cd "$temp_dir"
    if [[ "$extension" == "tar.gz" ]]; then
        tar -xzf "$filename" || {
            log_error "Failed to extract archive"
            rm -rf "$temp_dir"
            exit 1
        }
    else
        unzip -q "$filename" || {
            log_error "Failed to extract archive"
            rm -rf "$temp_dir"
            exit 1
        }
    fi
    
    # Find the binary
    local binary_name="apidrift"
    if [[ "$platform" == *"windows"* ]]; then
        binary_name="apidrift.exe"
    fi
    
    # Install the binary
    mkdir -p "$INSTALL_DIR"
    
    if [ -f "$binary_name" ]; then
        install -m 755 "$binary_name" "$INSTALL_DIR/apidrift" || {
            log_error "Failed to install binary to $INSTALL_DIR"
            rm -rf "$temp_dir"
            exit 1
        }
    else
        log_error "Binary not found in archive"
        rm -rf "$temp_dir"
        exit 1
    fi
    
    # Cleanup
    rm -rf "$temp_dir"
    
    log_info "✓ apidrift installed successfully to $INSTALL_DIR/apidrift"
}

# Verify installation
verify_installation() {
    if command -v apidrift &> /dev/null; then
        local version_output=$(apidrift --version 2>&1 || echo "unknown")
        log_info "✓ Installation verified: $version_output"
        log_info ""
        log_info "Run 'apidrift --help' to get started"
    else
        log_warn "apidrift was installed but is not in your PATH"
        log_info "Add $INSTALL_DIR to your PATH:"
        log_info "  echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.bashrc"
        log_info "  source ~/.bashrc"
    fi
}

# Main installation process
main() {
    log_info "Starting apidrift installation..."
    log_info ""
    
    # Detect platform
    platform=$(detect_platform)
    log_info "Detected platform: $platform"
    
    # Check permissions
    check_install_permissions
    
    # Install
    install_apidrift "$platform" "$VERSION"
    
    # Verify
    verify_installation
    
    log_info ""
    log_info "Installation complete! 🎉"
}

# Run main function
main

