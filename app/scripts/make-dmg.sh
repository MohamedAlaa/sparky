#!/bin/bash
# Builds a public Sparky.dmg: Sparky.app + an Applications shortcut.
# The public build never contains a Google OAuth client, account tokens or
# mail: every user connects their own Google Cloud client (docs/google-setup.md).
#
#   app/scripts/make-dmg.sh            -> app/dist/Sparky-<version>-<arch>.dmg
set -euo pipefail
cd "$(dirname "$0")/.."

[[ "$(uname -s)" == Darwin ]] || { echo "Building the DMG requires a Mac."; exit 1; }

version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
arch="$(uname -m)"
name="Sparky-$version-$arch"

echo "Building Sparky $version for $arch (no Google credentials)…"
env -u SPARKY_GOOGLE_CLIENT_ID -u SPARKY_GOOGLE_CLIENT_SECRET cargo build --release --locked

stage="$(mktemp -d "${TMPDIR:-/tmp}/sparky-dmg.XXXXXX")"
trap 'rm -rf "$stage"' EXIT
app="$stage/Sparky.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" dist
cp target/release/sparky "$app/Contents/MacOS/Sparky"
cp assets/Sparky.icns "$app/Contents/Resources/Sparky.icns"
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>Sparky</string>
<key>CFBundleIdentifier</key><string>com.begenius.sparky</string>
<key>CFBundleName</key><string>Sparky</string>
<key>CFBundleDisplayName</key><string>Sparky</string>
<key>CFBundleIconFile</key><string>Sparky.icns</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>$version</string>
<key>CFBundleVersion</key><string>$version</string>
<key>LSMinimumSystemVersion</key><string>11.0</string>
<key>LSApplicationCategoryType</key><string>public.app-category.productivity</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
/usr/bin/plutil -lint "$app/Contents/Info.plist" >/dev/null

# Guard: the public app must not carry anyone's OAuth client.
if [[ -e "$app/Contents/Resources/google-oauth.json" ]] \
   || strings "$app/Contents/MacOS/Sparky" | grep -qE '[0-9]+-[a-z0-9]{20,}\.apps\.googleusercontent\.com|GOCSPX-'; then
    echo "Refusing to package: a Google OAuth client is embedded in the build."; exit 1
fi

# Ad-hoc signature (required on Apple silicon). Not notarized: see README.
/usr/bin/codesign --force --deep --sign - "$app"
/usr/bin/codesign --verify --deep --strict "$app"

ln -s /Applications "$stage/Applications"
dmg="dist/$name.dmg"
rm -f "$dmg"
/usr/bin/hdiutil create -volname "Sparky" -srcfolder "$stage" -fs HFS+ -format UDZO -imagekey zlib-level=9 -ov "$dmg" >/dev/null
/usr/bin/hdiutil verify "$dmg" >/dev/null
shasum -a 256 "$dmg" | tee "$dmg.sha256"
echo "Ready: app/$dmg"
