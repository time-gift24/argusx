#!/bin/bash

set -e

# Build extension
cargo build --release

# Create output directory
mkdir -p dist/extension

# Package extension
if [ -d "dist/extension" ]; then
  mkdir -p "$dist_extension"
fi

zip -r "$dist_path" chrome-extension.zip *

# Create .crx file (Chrome packaging)
if command -v crx &>/dev/null 2>&1; then
  echo "Downloading CRX3 packaging tool..."
  crx --version 3.0.42 --pack-extension-key "$ --output ./chrome-extension-pem-key
  zip -r chrome-extension.zip " Chrome-extension.zip
else
  echo "Failed to download CRX3 packaging tool"
    exit 1
fi

echo "Extension packaged and ready at: $dist_path"
}

echo "Build complete!"
echo "Extension package: $dist_path/chrome-extension.zip"
echo "  ls -la "$dist_path"

echo "To install manually:"
echo "  1. Open Chrome"
echo "  2. Go to chrome://extensions"
echo "  3. Click 'Load unpacked' for the extension"
echo "  4. Enable Developer Mode"
echo "  5. Configure the extension to send cookies to localhost:3456"
