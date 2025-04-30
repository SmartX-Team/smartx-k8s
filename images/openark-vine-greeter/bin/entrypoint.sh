#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Greeting Screen

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# Try detecting Primary GPU
declare -g primary_dev="$($(dirname "$0")/gpu-primary.sh)"

is_cleaned_up='false'
function load_primary_gpu() {
    driver="$1"
    if [ "x${driver}" == 'x' ]; then
        echo '* Usage: cleanup [driver]' >&2
        exec false
    fi

    if [ "x${primary_dev}" != 'x' ]; then
        if [ "x${is_cleaned_up}" == 'xfalse' ]; then
            # Unload all the other GPU devices
            "$(dirname "$0")/gpu-load-except.sh" "${primary_dev}" 'vfio-pci' >&2

            # Reset graphics modules
            "$(dirname "$0")/gpu-unload-modules.sh" >&2

            # Reset Primary GPU device: We want to kick off VGA Arbiter
            "$(dirname "$0")/pci-reset.sh" "${primary_dev}" >&2
        fi

        # Load Primary GPU device
        "$(dirname "$0")/pci-load.sh" "${primary_dev}" "${driver}" >&2
        sleep 1 # Some GPU drivers (e.g. nouveau) need some time to finish init

        # Unload all USB devices
        "$(dirname "$0")/usb-load-all.sh" 'vfio-pci' >&2
    fi
}

function terminate() {
    # Stop app
    if [ "x${pid_firefox}" != 'x' ]; then
        kill "${pid_firefox}" || true
        wait "${pid_firefox}" || true
    fi

    # Stop Xorg
    if [ "x${pid_xorg}" != 'x' ]; then
        kill "${pid_xorg}" || true
        wait "${pid_xorg}" || true
    fi

    # Unload GPU driver from the Primary GPU
    load_primary_gpu 'vfio-pci'
    exec true
}

trap -- 'terminate' SIGINT
trap -- 'terminate' SIGTERM

# Load GPU driver in the Primary GPU
load_primary_gpu 'gpu'

# Check GPU drivers
if ! find /dev/dri -mindepth 1 -maxdepth 1 -name "card*" -type c >/dev/null; then
    echo 'INFO: Empty video' >&2
    exec true
fi
if ! find /dev/dri -mindepth 1 -maxdepth 1 -name "renderD*" -type c >/dev/null; then
    echo 'INFO: Empty renderer' >&2
    exec true
fi

# Configure XDG_RUNTIME_DIR
if [ "x${XDG_RUNTIME_DIR}" != 'x' ]; then
    mkdir -p "${XDG_RUNTIME_DIR}"
    chown -R "$(id -u):$(id -g)" "${XDG_RUNTIME_DIR}"
fi

# Patch xorg.conf
cp /opt/X11/xorg.conf.d/kiosk/* /etc/X11/xorg.conf.d/

# Configure X
if [ ! -f /etc/X11/xorg.conf ]; then
    X -configure
    ln -sf "${HOME}/xorg.conf.new" /etc/X11/xorg.conf
fi

# Open Xorg session
Xorg &
declare -ig pid_xorg="$!"

# Wait for Xorg to be ready
X11_SOCK="/tmp/.X11-unix/X$(echo "${DISPLAY}" | grep -Po '[0-9]+$')"
if [ ! -S "${X11_SOCK}" ]; then
    if ! ps --pid "${pid_xorg}" >/dev/null; then
        echo 'Xorg failed' >&2
        exec false
    fi
    sleep 1
fi
until timeout 1 xrandr --listactivemonitors >/dev/null; do
    sleep 0.1
done

echo "Finding displays..."
monitor="$(xrandr --current | grep ' connected ' | awk '{print $1}' | head -n1)"
if [ "x${monitor}" == "x" ]; then
    echo 'Display not found!' >&2
    exec true
fi

echo "Resize display..."
SCREEN_SIZE='800x600'
xrandr --output "${monitor}" --mode "${SCREEN_SIZE}" || true

# Open a greeter app
firefox \
    --first-startup \
    --private \
    --window-size "${SCREEN_SIZE}" \
    --kiosk "${URL}" &
declare -ig pid_firefox="$!"

wait "${pid_firefox}" || true
terminate
