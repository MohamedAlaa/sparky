#!/bin/bash
# Runs inside the build container: compiles Sparky and writes a .deb and a
# .tar.gz to dist/. Called by scripts/make-linux.sh.
set -euo pipefail
cd /src
version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
arch="$(dpkg --print-architecture)"   # amd64 / arm64
name="sparky_${version}_${arch}"

export CARGO_TARGET_DIR=/build/target
env -u SPARKY_GOOGLE_CLIENT_ID -u SPARKY_GOOGLE_CLIENT_SECRET cargo build --release --locked
bin="$CARGO_TARGET_DIR/release/sparky"
strip "$bin"

# Guard: the public app must not carry anyone's OAuth client.
if strings "$bin" | grep -qE '[0-9]+-[a-z0-9]{20,}\.apps\.googleusercontent\.com|GOCSPX-'; then
    echo "Refusing to package: a Google OAuth client is embedded in the build."; exit 1
fi

stage="$(mktemp -d)"
root="$stage/$name"
install -Dm755 "$bin" "$root/usr/bin/sparky"
install -Dm644 assets/Sparky.png "$root/usr/share/icons/hicolor/512x512/apps/sparky.png"
install -Dm644 assets/fonts/Inter-OFL.txt "$root/usr/share/doc/sparky/Inter-OFL.txt"
install -Dm644 /dev/stdin "$root/usr/share/applications/sparky.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Sparky
GenericName=Email
Comment=A calmer kind of email
Exec=sparky
Icon=sparky
Terminal=false
Categories=Network;Email;Office;
StartupWMClass=sparky
DESKTOP

# Portable tarball (same tree, no package manager needed).
mkdir -p dist
tar -C "$root/usr" -czf "dist/sparky-$version-linux-$arch.tar.gz" --transform "s,^\.,sparky-$version," .

# Debian package.
size="$(du -sk "$root/usr" | cut -f1)"
mkdir -p "$root/DEBIAN"
cat > "$root/DEBIAN/control" <<CONTROL
Package: sparky
Version: $version
Section: mail
Priority: optional
Architecture: $arch
Installed-Size: $size
Depends: libc6 (>= 2.35), libsqlite3-0, libgl1, libxkbcommon0, libxkbcommon-x11-0, curl, openssl, xdg-utils, libsecret-tools
Recommends: gnome-keyring
Maintainer: BeGenius <noreply@begenius.dev>
Description: Sparky - a calmer kind of email
 A native mail workspace for Gmail, Outlook and IMAP accounts.
CONTROL
dpkg-deb --root-owner-group --build "$root" "dist/$name.deb" >/dev/null
rm -rf "$stage"
(cd dist && sha256sum "$name.deb" "sparky-$version-linux-$arch.tar.gz" | tee -a SHA256SUMS-linux)
