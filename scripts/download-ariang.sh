#!/bin/bash
# Download AriaNg standard release and extract to frontend/ directory.
# Usage: ./download-ariang.sh [version]
# Example: ./download-ariang.sh 1.3.8

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
FRONTEND_DIR="$PROJECT_DIR/frontend"
VERSION="${1:-1.3.8}"

DOWNLOAD_URL="https://github.com/mayswind/AriaNg/releases/download/${VERSION}/AriaNg-${VERSION}.zip"
ZIP_FILE="/tmp/AriaNg-${VERSION}.zip"

echo "==> Downloading AriaNg v${VERSION}..."
echo "    URL: $DOWNLOAD_URL"

# Download
if command -v curl &>/dev/null; then
    curl -fSL -o "$ZIP_FILE" "$DOWNLOAD_URL"
elif command -v wget &>/dev/null; then
    wget -q -O "$ZIP_FILE" "$DOWNLOAD_URL"
else
    echo "Error: curl or wget is required"
    exit 1
fi

# Clean and extract
echo "==> Extracting to $FRONTEND_DIR..."
rm -rf "$FRONTEND_DIR"
mkdir -p "$FRONTEND_DIR"
unzip -q -o "$ZIP_FILE" -d "$FRONTEND_DIR"

# Verify
if [ ! -f "$FRONTEND_DIR/index.html" ]; then
    echo "Error: index.html not found after extraction!"
    exit 1
fi

# Cleanup
rm -f "$ZIP_FILE"

echo "==> AriaNg v${VERSION} installed successfully to $FRONTEND_DIR"
ls -la "$FRONTEND_DIR"
