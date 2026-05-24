#!/bin/bash
set -e
cd "$(dirname "$0")"

echo "Building..."
cargo build

APP="The Room.app"
mkdir -p "$APP/Contents/MacOS"
cp target/debug/the_room "$APP/Contents/MacOS/the_room"

cat > "$APP/Contents/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>the_room</string>
    <key>CFBundleIdentifier</key>
    <string>com.theroom.game</string>
    <key>CFBundleName</key>
    <string>The Room</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
</dict>
</plist>
EOF

echo "Launching..."
open "$APP"
