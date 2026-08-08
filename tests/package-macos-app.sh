#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
temp_dir=$(mktemp -d)
trap 'rm -rf "$temp_dir"' EXIT

fixture_binary="$temp_dir/ludusavi"
printf '#!/usr/bin/env bash\nprintf "fixture\\n"\n' > "$fixture_binary"
chmod +x "$fixture_binary"

staging_dir="$temp_dir/staging"
archive="$temp_dir/ludusavi-v1.2.3-mac.tar.gz"

"$repo_root/scripts/package-macos-app.sh" "$fixture_binary" 1.2.3 "$staging_dir" "$archive"

app="$staging_dir/Ludusavi.app"
plist="$app/Contents/Info.plist"

test -x "$app/Contents/MacOS/Ludusavi"
test -f "$app/Contents/Resources/Ludusavi.icns"
test -L "$staging_dir/ludusavi"
test "$(readlink "$staging_dir/ludusavi")" = "Ludusavi.app/Contents/MacOS/Ludusavi"
test "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$plist")" = "Ludusavi"
test "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$plist")" = "com.mtkennerly.ludusavi"
test "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$plist")" = "1.2.3"
test "$(tar --list --gzip --file="$archive" | rg '^Ludusavi\.app/Contents/MacOS/Ludusavi$')" = "Ludusavi.app/Contents/MacOS/Ludusavi"
test "$(tar --list --gzip --file="$archive" | rg '^ludusavi$')" = "ludusavi"

extracted_dir="$temp_dir/extracted"
mkdir "$extracted_dir"
tar --extract --gzip --file="$archive" --directory="$extracted_dir"
test -L "$extracted_dir/ludusavi"
test "$(readlink "$extracted_dir/ludusavi")" = "Ludusavi.app/Contents/MacOS/Ludusavi"
