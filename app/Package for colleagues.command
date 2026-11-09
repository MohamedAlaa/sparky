#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"

if [[ "$(uname -s)" != Darwin ]]; then
    echo "This packager requires a Mac."
    exit 1
fi

echo "Building Sparky for this Mac's processor…"
cargo build --release --offline --locked

arch="$(uname -m)"
stage="$(mktemp -d "${TMPDIR:-/tmp}/sparky-package.XXXXXX")"
trap 'rm -rf "$stage"' EXIT
bundle="$stage/Sparky.app"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources" dist
cp assets/Sparky.png dist/Sparky.png
cp assets/Sparky.icns dist/Sparky.icns
# Desktop OAuth configuration belongs to the app; account tokens never do.
google_config="$HOME/Library/Application Support/Sparky/google-oauth.json"
if [[ -f "$google_config" ]]; then
    cp "$google_config" "$bundle/Contents/Resources/google-oauth.json"
    echo "Including the Google Desktop app registration for browser sign-in."
elif [[ -z "${SPARKY_GOOGLE_CLIENT_ID:-}" || -z "${SPARKY_GOOGLE_CLIENT_SECRET:-}" ]]; then
    echo "Google sign-in is not configured. Use Set up Google sign-in in Sparky before sharing for Gmail testing."
fi
cp assets/Sparky.icns "$bundle/Contents/Resources/Sparky.icns"
cp target/release/sparky "$bundle/Contents/MacOS/Sparky"
chmod 755 "$bundle/Contents/MacOS/Sparky"
cat > "$bundle/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>Sparky</string>
<key>CFBundleIdentifier</key><string>com.begenius.sparky</string>
<key>CFBundleName</key><string>Sparky</string>
<key>CFBundleIconFile</key><string>Sparky.icns</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.1.0</string>
<key>CFBundleVersion</key><string>1</string>
<key>LSMinimumSystemVersion</key><string>11.0</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
/usr/bin/plutil -lint "$bundle/Contents/Info.plist"
# Ad-hoc signing supports local testing; this is not Developer ID notarization.
/usr/bin/codesign --force --sign - "$bundle"
/usr/bin/codesign --verify --deep --strict "$bundle"
cp TESTING.txt "$stage/START HERE.txt"
cp 'GOOGLE ACCESS FIX.txt' "$stage/GOOGLE ACCESS FIX.txt"
ln -s /Applications "$stage/Applications"
archive="$PWD/dist/Sparky-macOS-$arch.zip"
/usr/bin/ditto -c -k --keepParent "$bundle" "$archive"
# Preserve the Applications shortcut as a symlink; never archive its contents.
(cd "$stage" && /usr/bin/zip -q -y "$archive" "START HERE.txt" "GOOGLE ACCESS FIX.txt" Applications)
/usr/bin/unzip -tq "$archive"
echo "Ready to share: $archive"
echo "Send this ZIP to colleagues using the same Mac processor type ($arch)."
echo "No DMG is needed: unzip, then drag Sparky.app onto the Applications shortcut."
