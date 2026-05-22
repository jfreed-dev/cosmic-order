#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
#
# vm-test.sh — build, deploy, run, and screenshot COSMIC ORDER on the COSMIC VM.
#
# The homelab build hosts are arm64 but the test VM is amd64, so the package is
# built here (x86_64) in the project's noble Docker image, copied to the VM,
# installed, launched in the live COSMIC session, screenshotted, and the image
# is pulled back to this host.
#
# Requirements: docker on this host; SSH key access to the VM.
#
# Config (env):
#   VM         SSH target               (default: jon@10.10.84.118)
#   BUILD_DIR  scratch build directory  (default: /tmp/cobuild)
#   OUT_DIR    where shots land locally (default: /tmp/cosmic-vm)
#
# Usage:
#   scripts/vm-test.sh             build + deploy + screenshot
#   scripts/vm-test.sh --no-build  reuse the last .deb; just deploy + screenshot
set -euo pipefail

VM="${VM:-jon@10.10.84.118}"
BUILD_DIR="${BUILD_DIR:-/tmp/cobuild}"
OUT_DIR="${OUT_DIR:-/tmp/cosmic-vm}"
IMAGE="cosmic-order-deb-builder:noble"
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SSH=(ssh -o BatchMode=yes)
SCP=(scp -q -o BatchMode=yes)

log() { printf '\033[0;36m[vm-test]\033[0m %s\n' "$*"; }
die() { printf '\033[0;31m[vm-test] %s\033[0m\n' "$*" >&2; exit 1; }

build_deb() {
    log "Syncing $REPO -> $BUILD_DIR/cosmic-order"
    # Docker may have left root-owned files from a previous run.
    sudo chown -R "$(id -u):$(id -g)" "$BUILD_DIR" 2>/dev/null || true
    mkdir -p "$BUILD_DIR/cosmic-order"
    rsync -a --delete \
        --exclude 'target/' --exclude '.git/' --exclude 'vendor/' \
        --exclude 'vendor.tar' --exclude '.cargo/' --exclude 'dist/' \
        "$REPO/" "$BUILD_DIR/cosmic-order/"

    log "Building deb-builder image (cached after first run)"
    docker build -q -f "$BUILD_DIR/cosmic-order/scripts/Dockerfile.deb-builder" \
        -t "$IMAGE" "$BUILD_DIR/cosmic-order/scripts" >/dev/null

    log "Building amd64 .deb"
    docker run --rm --user "$(id -u):$(id -g)" \
        -v "$BUILD_DIR:/build" -w /build/cosmic-order -e HOME=/tmp \
        "$IMAGE" dpkg-buildpackage -us -uc -b -d
}

if [ "${1:-}" = "--no-build" ]; then
    log "--no-build: reusing the existing .deb"
else
    build_deb
fi

DEB="$(ls -t "$BUILD_DIR"/cosmic-order_*_amd64.deb 2>/dev/null | head -1)"
[ -n "$DEB" ] || die "no amd64 .deb in $BUILD_DIR (run without --no-build first)"
log "Package: $(basename "$DEB")"

log "Deploying to $VM"
"${SCP[@]}" "$DEB" "$VM:/tmp/"
"${SSH[@]}" "$VM" "sudo dpkg -i /tmp/$(basename "$DEB") >/dev/null 2>&1 || sudo apt-get -f install -y >/dev/null 2>&1; printf 'installed: '; cosmic-order --version"

log "Launching GUI + capturing screenshot"
mkdir -p "$OUT_DIR"
# Derive the live session's Wayland/DBus env from the runtime dir, launch the
# GUI detached, give it a moment to render, then screenshot via the COSMIC
# portal. LIBGL_ALWAYS_SOFTWARE/llvmpipe keeps GL happy on the GPU-less VM.
SHOT="$("${SSH[@]}" "$VM" '
    pkill -x cosmic-order 2>/dev/null || true
    sleep 1
    RT="/run/user/$(id -u)"
    WL="$(ls "$RT" 2>/dev/null | grep -m1 -E "^wayland-[0-9]+$")"
    export XDG_RUNTIME_DIR="$RT" XDG_CURRENT_DESKTOP=COSMIC
    export DBUS_SESSION_BUS_ADDRESS="unix:path=$RT/bus"
    export WAYLAND_DISPLAY="$WL"
    export LIBGL_ALWAYS_SOFTWARE=1 GALLIUM_DRIVER=llvmpipe
    nohup cosmic-order >/tmp/cosmic-order.log 2>&1 & disown
    sleep 8
    timeout 20 cosmic-screenshot --interactive=false --notify=false -s /tmp 2>/dev/null | tail -1
')"
[ -n "$SHOT" ] || die "no screenshot produced — is the VM display awake and unlocked?"
log "Captured on VM: $SHOT"
"${SCP[@]}" "$VM:$SHOT" "$OUT_DIR/"
log "Pulled -> $OUT_DIR/$(basename "$SHOT")"
