#!/bin/bash
# Download aria2c binary for a specific platform and place it in src-tauri/binaries/.
# Usage: ./download-aria2.sh <target-triple>
# Example: ./download-aria2.sh x86_64-unknown-linux-gnu
#          ./download-aria2.sh x86_64-apple-darwin
#          ./download-aria2.sh aarch64-apple-darwin

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BINARIES_DIR="$PROJECT_DIR/src-tauri/binaries"

TARGET="${1:-}"
ARIA2_VERSION="${2:-1.37.0}"

if [ -z "$TARGET" ]; then
    # Auto-detect target triple
    ARCH=$(uname -m)
    OS=$(uname -s)
    case "$OS" in
        Linux)
            case "$ARCH" in
                x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
                aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
                *) echo "Unsupported arch: $ARCH"; exit 1 ;;
            esac
            ;;
        Darwin)
            case "$ARCH" in
                x86_64) TARGET="x86_64-apple-darwin" ;;
                arm64) TARGET="aarch64-apple-darwin" ;;
                *) echo "Unsupported arch: $ARCH"; exit 1 ;;
            esac
            ;;
        *)
            echo "Unsupported OS: $OS (use download-aria2.ps1 for Windows)"
            exit 1
            ;;
    esac
fi

OUTPUT_FILE="$BINARIES_DIR/aria2c-$TARGET"

echo "==> Downloading aria2 v${ARIA2_VERSION} for $TARGET..."

mkdir -p "$BINARIES_DIR"

case "$TARGET" in
    x86_64-unknown-linux-gnu|aarch64-unknown-linux-gnu)
        # For Linux, try to use the system package manager or download static build
        if command -v apt-get &>/dev/null; then
            echo "    Trying to get aria2c from system..."
            ARIA2_PATH=$(which aria2c 2>/dev/null || true)
            if [ -z "$ARIA2_PATH" ]; then
                echo "    aria2c not found in PATH. Installing via apt..."
                sudo apt-get update -qq && sudo apt-get install -y -qq aria2
                ARIA2_PATH=$(which aria2c)
            fi
            cp "$ARIA2_PATH" "$OUTPUT_FILE"
        else
            echo "    Downloading static aria2c build..."
            # Download from GitHub releases - static build
            DOWNLOAD_URL="https://github.com/aria2/aria2/releases/download/release-${ARIA2_VERSION}/aria2-${ARIA2_VERSION}-${ARCH}-linux-gnu.tar.bz2"
            ARCH_MAP="x86_64"
            if [ "$TARGET" = "aarch64-unknown-linux-gnu" ]; then
                ARCH_MAP="aarch64"
            fi
            TMP_DIR=$(mktemp -d)
            curl -fSL "https://github.com/q3aql/aria2-static-builds/releases/download/v${ARIA2_VERSION}/aria2-${ARIA2_VERSION}-linux-gnu-${ARCH_MAP}-build1.tar.bz2" -o "$TMP_DIR/aria2.tar.bz2" || {
                echo "    Static build download failed. Please install aria2c manually and copy to $OUTPUT_FILE"
                exit 1
            }
            tar -xjf "$TMP_DIR/aria2.tar.bz2" -C "$TMP_DIR"
            find "$TMP_DIR" -name "aria2c" -type f -exec cp {} "$OUTPUT_FILE" \;
            rm -rf "$TMP_DIR"
        fi
        ;;
    x86_64-apple-darwin|aarch64-apple-darwin)
        # For macOS, use Homebrew or download from releases
        ARIA2_PATH=$(which aria2c 2>/dev/null || true)
        if [ -z "$ARIA2_PATH" ]; then
            if command -v brew &>/dev/null; then
                echo "    Installing aria2 via Homebrew..."
                brew install aria2
                ARIA2_PATH=$(which aria2c)
            else
                echo "    Error: aria2c not found. Please install via: brew install aria2"
                exit 1
            fi
        fi
        cp "$ARIA2_PATH" "$OUTPUT_FILE"
        ;;
    *)
        echo "Unsupported target: $TARGET"
        exit 1
        ;;
esac

chmod +x "$OUTPUT_FILE"
echo "==> aria2c binary placed at: $OUTPUT_FILE"
file "$OUTPUT_FILE"
