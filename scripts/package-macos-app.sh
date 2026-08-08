#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 4 ]; then
    echo "Usage: $0 <binary> <version> <staging-dir> <archive>" >&2
    exit 64
fi

binary=$1
version=$2
staging_dir=$3
archive=$4
repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
app="$staging_dir/Ludusavi.app"

test -x "$binary"
test -f "$repo_root/assets/icon.icns"

rm -rf "$staging_dir"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$binary" "$app/Contents/MacOS/Ludusavi"
cp "$repo_root/assets/icon.icns" "$app/Contents/Resources/Ludusavi.icns"
ln -s "Ludusavi.app/Contents/MacOS/Ludusavi" "$staging_dir/ludusavi"

cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleDisplayName</key>
    <string>Ludusavi</string>
    <key>CFBundleExecutable</key>
    <string>Ludusavi</string>
    <key>CFBundleIconFile</key>
    <string>Ludusavi.icns</string>
    <key>CFBundleIdentifier</key>
    <string>com.mtkennerly.ludusavi</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>Ludusavi</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${version}</string>
    <key>CFBundleVersion</key>
    <string>${version}</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST

tar --create --gzip --file="$archive" --directory="$staging_dir" Ludusavi.app ludusavi
