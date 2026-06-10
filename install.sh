#!/bin/sh
set -e

echo "Rewind Installer for Unix/macOS"

REPO="Chronos778/git-rewind"
API_URL="https://api.github.com/repos/$REPO/releases/latest"

echo "Fetching latest release from $REPO..."
RELEASE=$(curl -s $API_URL)

ARCH=$(uname -m)
case "$ARCH" in
    x86_64|amd64) ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
    *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
case "$OS" in
    linux) OS="linux" ;;
    darwin) OS="apple-darwin" ;;
    *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

# Poor man's JSON parsing (assuming standard github API format)
URL=$(echo "$RELEASE" | grep -o "browser_download_url\": \"[^\"]*" | grep "$OS" | grep "$ARCH" | grep "tar.gz" | cut -d'"' -f3 | head -n 1)

if [ -z "$URL" ]; then
    echo "Failed to find a suitable download for $OS-$ARCH."
    exit 1
fi

TEMP_DIR=$(mktemp -d)
TEMP_TAR="$TEMP_DIR/rewind.tar.gz"

echo "Downloading $URL..."
curl -L -s -o "$TEMP_TAR" "$URL"

echo "Downloading checksum..."
if curl -fsSL -o "$TEMP_TAR.sha256" "$URL.sha256"; then
    echo "Verifying checksum..."
    EXPECTED_CHECKSUM=$(cat "$TEMP_TAR.sha256" | awk '{print $1}')
    if command -v sha256sum >/dev/null 2>&1; then
        ACTUAL_CHECKSUM=$(sha256sum "$TEMP_TAR" | awk '{print $1}')
    elif command -v shasum >/dev/null 2>&1; then
        ACTUAL_CHECKSUM=$(shasum -a 256 "$TEMP_TAR" | awk '{print $1}')
    else
        echo "Warning: Neither sha256sum nor shasum found. Skipping checksum verification."
        ACTUAL_CHECKSUM=$EXPECTED_CHECKSUM
    fi

    if [ "$ACTUAL_CHECKSUM" != "$EXPECTED_CHECKSUM" ]; then
        echo "Error: Checksum verification failed!"
        echo "Expected: $EXPECTED_CHECKSUM"
        echo "Actual:   $ACTUAL_CHECKSUM"
        rm -rf "$TEMP_DIR"
        exit 1
    fi
else
    echo "Warning: Could not download checksum file. Skipping verification."
fi

INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

echo "Extracting to $INSTALL_DIR..."
tar -xzf "$TEMP_TAR" -C "$INSTALL_DIR"
rm -rf "$TEMP_DIR"

if [ ! -x "$INSTALL_DIR/rewind" ]; then
    echo "Extraction failed: rewind executable not found."
    exit 1
fi

echo "[SUCCESS] Installation Complete!"
echo "Binary installed to $INSTALL_DIR/rewind."

case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *) 
      echo -e "\033[1;33mNote: $INSTALL_DIR is not in your PATH.\033[0m"
      echo -e "You might want to add 'export PATH=\"\$HOME/.local/bin:\$PATH\"' to your ~/.bashrc or ~/.zshrc."
      ;;
esac
