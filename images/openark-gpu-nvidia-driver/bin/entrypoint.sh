#!/usr/bin/env bash
# Copyright (c) 2018-2020, NVIDIA CORPORATION. All rights reserved.
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

# Configure default variables
KERNEL_CONF_HOME='/drivers'
KERNEL_VERSION="$(uname -r)"
PID_FILE="${RUN_DIR}/${0##*/}.pid"
USE_HOST_KERNEL="${USE_HOST_KERNEL:-"true"}"

DAEMON_LIST=(
    'nvidia-persistenced/nvidia-persistenced'
    'nvidia-gridd/nvidia-gridd'
    'nvidia-fabricmanager/nv-fabricmanager'
)

# NOTE: Ordered by unloading
KERNEL_MODULE_LIST=(
    "nvidia_drm"
    "nvidia_modeset"
    "nvidia_uvm"
    "nvidia_peermem"
    "nvidia"
)

# Install the kernel modules header/builtin/order files and generate the kernel version string.
function _install_dependencies() (
    local dst_dir="/lib/modules/${KERNEL_VERSION}"
    local tmp_dir=$(mktemp -d)

    # Cleanup while being halted
    trap "rm -rf ${tmp_dir}" EXIT
    cd "${tmp_dir}"

    # Create a virtual module proc directory
    rm -rf "${dst_dir}"
    mkdir -p "${dst_dir}/proc"

    if [ "x${USE_HOST_KERNEL}" == 'xtrue' ] && [ -d "/host/usr/src/linux-headers-${KERNEL_VERSION}" ]; then
        echo 'Using host Linux kernel headers...'
        cp -ar "/host${dst_dir}/build" "${dst_dir}"
        for path in $(find /host/usr/src -maxdepth 1 -mindepth 1 -type d); do
            ln -sf "${path}" "$(echo "${path}" | grep -Po '^/host\K.*$')"
        done
    else
        echo 'Installing Linux kernel headers...'
        apt-get update
        apt-get install -y --no-install-recommends \
            "linux-headers-${KERNEL_VERSION}" \
            >/dev/null
    fi

    if [ "x${USE_HOST_KERNEL}" == 'xtrue' ] && [ -d "/host${dst_dir}/kernel" ]; then
        echo 'Using host Linux kernel module files...'
        for path in $(find "/host${dst_dir}" -maxdepth 1 -mindepth 1 -name 'modules.*' -type f); do
            ln -sf "${path}" "$(echo "${path}" | grep -Po '^/host\K.*$')"
        done
        cp -r "/host${dst_dir}/kernel" "${dst_dir}/kernel"
    else
        echo 'Downloading Linux kernel module files...'
        apt-get download -y "linux-image-${KERNEL_VERSION}"
        dpkg -x ./linux-image-*.deb .
        apt-get download -y "linux-modules-${KERNEL_VERSION}"
        dpkg -x ./linux-modules-*.deb .
        rm ./linux-*.deb

        echo 'Installing Linux kernel module files...'
        mv .${dst_dir}/modules.* "${dst_dir}"
        mv .${dst_dir}/kernel "${dst_dir}"
    fi
    depmod "${KERNEL_VERSION}"

    echo 'Generating Linux kernel version string...'
    if [ ! -d ./boot ] && [ -d /host/boot ]; then
        ln -sf /host/boot ./boot
    fi
    ls -1 ./boot/vmlinuz-* | sed 's/\/boot\/vmlinuz-//g' - >version
    if [ -z "$(<version)" ]; then
        echo "Could not locate Linux kernel version string" >&2
        return 1
    fi
    mv version "${dst_dir}/proc"

    # Cleanup
    rm -rf "${tmp_dir}"
)

# Load a kernel module with specific parameters
function _load_kernel_module() {
    local name="$1"
    local path="${KERNEL_CONF_HOME}/${name}.conf"
    local params=()

    # Load given kernel module parameters
    if [ -f "${path}" ]; then
        while IFS="" read -r param || [ -n "${param}" ]; do
            params+=("$param")
        done <"${path}"
        echo "- module/${name}/parameters = ${params[@]}"
    fi

    # Load module
    echo "- ${name}/load = Init"
    set -o xtrace +o nounset
    modprobe "${name}" "${params[@]}"
    set +o xtrace -o nounset
    echo "- module/${name}/load = Running"
}

# ( Build | Unpack ) and install the driver.
function _build_driver() {
    local args=()

    # Parse arguments
    if [ "${ACCEPT_LICENSE}" == "true" ]; then
        args+=("--accept-license")
    fi
    if [ -n "${MAX_THREADS}" ]; then
        args+=("--concurrency-level=${MAX_THREADS}")
    fi

    # Select driver kernel module type
    if [[ "${KERNEL_MODULE_TYPE}" == "open" || "${KERNEL_MODULE_TYPE}" == "proprietary" ]]; then
        [[ "${KERNEL_MODULE_TYPE}" == "open" ]] && kernel_type=kernel-open || kernel_type=kernel
        echo "Proceeding with user-specified kernel module type ${KERNEL_MODULE_TYPE}"
        args+=("-m=${kernel_type}")
    fi

    # Complete building arguments
    echo "- installer/args = ${args[@]}"

    # Install driver
    sh ./installer --silent \
        --ui=none \
        --no-backup \
        --no-check-for-alternate-installs \
        --no-nouveau-check \
        --no-nvidia-modprobe \
        --no-rpms \
        ${args[@]}
}

# Load firmwares
function _load_firmware() {
    local fw_config_file="/sys/module/firmware_class/parameters/path"
    local fw_home="${RUN_DIR}/driver/lib/firmware"

    echo "Configuring the following firmware search path in '${fw_config_file}': ${fw_home}"
    if [[ ! -z $(grep '[^[:space:]]' "${fw_config_file}") ]]; then
        echo "WARNING: A search path is already configured in ${fw_config_file}"
        echo "         Retaining the current configuration"
    else
        echo -n "${fw_home}" >"${fw_config_file}" || echo "WARNING: Failed to configure firmware search path"
    fi
}

# Load the kernel modules and start persistenced.
function _load_driver() {
    # Collect pending kernel modules
    local modules=("nvidia" "nvidia-uvm" "nvidia-modeset" "nvidia-drm")
    if [ "x${GPU_DIRECT_RDMA_ENABLED}" == 'xtrue' ]; then
        modules+=('nvidia-peermem')
    fi

    # Load required kernel modules
    modprobe -a ${KERNEL_MODULE_DEPS}

    # Load kernel modules
    for name in ${modules[@]}; do
        echo "- ${name}/load = Pending"
        _load_kernel_module "${name}" || true
    done

    # Load daemon: nvidia-persistenced
    echo '- daemon/nvidia-persistenced/nvidia-persistenced/load = Init'
    nvidia-persistenced --persistence-mode
    echo '- daemon/nvidia-persistenced/nvidia-persistenced/load = Running'

    # Load daemon: nvidia-fabricmanager
    if [ -d /proc/driver/nvidia-nvswitch/devices ] &&
        [ ! -z "$(ls -A /proc/driver/nvidia-nvswitch/devices)" ]; then
        echo '- daemon/nvidia-fabricmanager/nv-fabricmanager/load = Init'
        nv-fabricmanager -c /usr/share/nvidia/nvswitch/fabricmanager.cfg
        echo '- daemon/nvidia-fabricmanager/nv-fabricmanager/load = Running'
    else
        echo '- daemon/nvidia-fabricmanager/nv-fabricmanager/load = Skipped'
    fi
}

# Stop daemons and unload the kernel modules if they are currently loaded.
function _unload_driver() {
    # Stop daemons
    for name in ${DAEMON_LIST[@]}; do
        local pid_file="/var/run/${name}.pid"
        if [ -f "${pid_file}" ]; then
            echo "- daemon/${name}/load: Terminating"
            local pid=$(<"${pid_file}")

            kill -SIGTERM "${pid}" || true
            until [ ! -d "/proc/${pid}" ]; do
                kill -0 "${pid}" 2>/dev/null || true
                sleep 0.1
            done
            echo "- daemon/${name}/load: Terminated"
        fi
    done

    # Unload kernel modules
    for name in ${KERNEL_MODULE_LIST[@]}; do
        refcnt_file="/sys/module/${name}/refcnt"
        if [ -f "${refcnt_file}" ]; then
            while [ -f "${refcnt_file}" ] && [ $(<"${refcnt_file}") -gt 0 ]; do
                echo "- module/${name}/load: Terminating (Still in use)"
                grep -l 'nvidia' /proc/*/maps | awk -F '/' '{print $3}' | sort -u | xargs kill
                sleep 0.1
            done
            echo "- module/${name}/load: Terminating"
            rmmod "${name}"
            echo "- module/${name}/load: Terminated"
        else
            echo "- module/${name}/load: Skipped"
        fi
    done

    # Unload the other kernel modules
    for pci_id in $(ls /sys/bus/pci/devices); do
        dev="/sys/bus/pci/devices/${pci_id}"
        # Select NVIDIA products
        if [ ! -f "${dev}/vendor" ] || ! cat "${dev}/vendor" | grep -Posq '^0x10de$'; then
            continue
        fi
        # Select GPUs and Audio devices
        # - GPU: 0x030000
        # - Audio: 0x040300
        if [ ! -f "${dev}/class" ] || ! cat "${dev}/class" | grep -Posq '^(0x030000|0x040300)$'; then
            continue
        fi
        # Completed filtering devices
        if [ ! -L "${dev}/driver" ]; then
            echo "- device/${pci_id}/load: Skipped"
            continue
        fi
        # Unload driver
        echo "- device/${pci_id}/load: Terminating"
        echo >"${dev}/driver_override"
        echo "${pci_id}" >"${dev}/driver/unbind"
        # Enable device
        echo "- device/${pci_id}/load: Terminating (Reloading)"
        until [ "x$(cat "${dev}/enable")" == 'x1' ]; do
            echo 1 >"${dev}/enable" 2>/dev/null || true
            sleep 0.2
        done
        echo "- device/${pci_id}/load: Terminated"
    done
}

# Mount the driver rootfs into the run directory with the exception of sysfs.
function _mount_rootfs() {
    echo 'Mounting NVIDIA driver rootfs...'
    mount --make-runbindable /sys
    mount --make-private /sys
    mkdir -p "${RUN_DIR}/driver"
    mount --rbind / "${RUN_DIR}/driver"
}

# Unmount the driver rootfs from the run directory.
function _unmount_rootfs() {
    echo 'Unmounting NVIDIA driver rootfs...'
    if findmnt -r -o TARGET | grep "${RUN_DIR}/driver" >/dev/null; then
        umount -l -R "${RUN_DIR}/driver"
    fi
}

function _shutdown() {
    if _unload_driver; then
        _unmount_rootfs
        rm -f "${PID_FILE}"
        return 0
    fi
    return 1
}

function _shutdown_fail() {
    if [ -f '/var/log/nvidia-installer.log' ]; then
        cat '/var/log/nvidia-installer.log'
    fi
    _shutdown
}

function main() {
    # Check whether the driver instance already exists
    exec 3>"${PID_FILE}"
    if ! flock -n 3; then
        echo "An instance of the driver is already running, aborting"
        exit 1
    fi
    echo $$ >&3

    # Revert loading kernel modules if halted while loading
    trap "echo 'Caught signal'; exit 1" HUP INT QUIT PIPE TERM
    trap "_shutdown_fail" EXIT

    # Cleanup old driver
    _unload_driver
    _unmount_rootfs

    # Install dependencies
    _install_dependencies

    # Build a new driver
    _build_driver

    # Load the new driver
    _load_firmware
    _load_driver
    _mount_rootfs

    echo 'Done, now waiting for signal'
    sleep infinity &
    trap "echo 'Caught signal'; _shutdown && { kill $!; exit 0; }" HUP INT QUIT PIPE TERM
    trap - EXIT
    while true; do wait $! || continue; done
    exit 0
}

main
