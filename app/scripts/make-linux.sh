#!/bin/bash
# Builds Sparky for Ubuntu (22.04+) in Docker, from macOS or Linux.
# The public build never contains a Google OAuth client (docs/google-setup.md).
#
#   app/scripts/make-linux.sh               -> amd64 (most PCs)
#   app/scripts/make-linux.sh arm64         -> ARM (e.g. Raspberry Pi, ARM VMs)
#   app/scripts/make-linux.sh amd64 arm64   -> both
# Output: app/dist/sparky_<version>_<arch>.deb and sparky-<version>-linux-<arch>.tar.gz
set -euo pipefail
cd "$(dirname "$0")/.."
command -v docker >/dev/null || { echo "Docker is required."; exit 1; }
archs=("${@:-amd64}")
rm -f dist/SHA256SUMS-linux
for arch in "${archs[@]}"; do
    case "$arch" in amd64|arm64) ;; *) echo "Unknown architecture: $arch (use amd64 or arm64)"; exit 1;; esac
    image="sparky-linux-build:$arch"
    echo "Building Sparky for Ubuntu ${arch}…"
    docker build --platform "linux/$arch" -t "$image" scripts/linux
    # Source is mounted read-only apart from dist/; build output stays in a volume.
    mkdir -p dist
    docker run --rm --platform "linux/$arch" \
        -v "$PWD:/src:ro" -v "$PWD/dist:/src/dist" \
        -v "sparky-cargo-$arch:/opt/cargo/registry" -v "sparky-target-$arch:/build" \
        "$image" bash scripts/linux/package.sh
done
echo "Ready in app/dist/"
