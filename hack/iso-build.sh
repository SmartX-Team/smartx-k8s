#!/usr/bin/env bash
# Copyright (c) 2025-2026 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

###########################################################
#   Configuration                                         #
###########################################################

export ROOT="${ROOT:-$(pwd)}"

IMAGE_HOME="$($(dirname "$0")/iso-prepare.sh)"
cd "${IMAGE_HOME}/openark-kiss/templates"

ARCH="${ARCH:-amd64}"
OS_BASE_URL="$(cat 'configmap-iso.yaml' | yq -r '.data.os_base_url')"
OS_DIST="$(cat 'configmap-kiss-config.yaml' | yq -r '.data.os_dist')"
OS_REVISION="$(cat 'configmap-iso.yaml' | yq -r '.data.os_revision')"
OS_VERSION="$(cat 'configmap-kiss-config.yaml' | yq -r '.data.os_version')"

OS_SRC="/tmp/${OS_DIST}-${OS_VERSION}.${OS_REVISION}-${ARCH}-base"
OS_SRC_FILE="${ROOT}/iso/${OS_DIST}-${OS_VERSION}.${OS_REVISION}-${ARCH}-base.iso"
OS_TGT_FILE="${ROOT}/iso/${OS_DIST}-${OS_VERSION}.${OS_REVISION}-${ARCH}-smartx.iso"

###########################################################
#   Build an ISO image                                    #
###########################################################

# Download a base ISO
case "${OS_DIST}" in
'ubuntu')
    INSTALL_KIND='cloud-init'
    OS_URL="${OS_BASE_URL}/${OS_DIST}-${OS_VERSION}.${OS_REVISION}-live-server-${ARCH}.iso"
    ;;
*)
    echo "Unsupported OS: ${OS_DIST}" >&2
    exec false
    ;;
esac

if [ -f "${OS_SRC_FILE}" ]; then
    echo "* Using downloaded ISO: ${OS_SRC_FILE}"
else
    echo "* Downloading ISO: ${OS_URL}"
    curl -Lo "${OS_SRC_FILE}" "${OS_URL}"
fi

echo "* Extracting ISO image: ${OS_SRC}"
sudo rm -rf "${OS_SRC}"
sudo mkdir -p "${OS_SRC}"
sudo 7z -y x "${OS_SRC_FILE}" -x'![BOOT]' -o"${OS_SRC}" >/dev/null
cd "${OS_SRC}"

case "${INSTALL_KIND}" in
'cloud-init')
    # Create empty meta-data file
    sudo mkdir -p ./autoinstall
    sudo touch './autoinstall/meta-data'

    # Copy user-data file
    cat "${IMAGE_HOME}/openark-kiss/templates/configmap-assets.yaml" |
        yq "select(.metadata.name == \"assets-boot\") | .data.\"cloud-init_${OS_DIST}_${OS_VERSION}.yaml\"" |
        sudo tee './autoinstall/user-data' >/dev/null

    # Copy SmartX K8S
    (
        sudo mkdir './autoinstall/smartx-k8s'
        cd './autoinstall/smartx-k8s'

        # Copy application
        sudo cp -Lr "${ROOT}/templates" './'
        sudo cp "${ROOT}/.helmignore" './'
        sudo cp "${ROOT}/Chart.yaml" './'
        sudo cp "${ROOT}/LICENSE" './'
        sudo cp "${ROOT}/manifest.yaml" './'
        sudo cp "${ROOT}/README.md" './'
        sudo cp "${ROOT}/values.yaml" './'

        # Copy apps
        sudo cp -Lr "${ROOT}/apps" './'

        # Copy hack
        sudo cp -Lr "${ROOT}/hack" './'

        # Copy patches
        sudo mkdir './patches'
        for name in $(
            cat "${IMAGE_HOME}/openark-kiss/templates/configmap-assets.yaml" |
                yq -r 'select(.metadata.name == "assets-patches") | .data | keys | .[]'
        ); do
            cat "${IMAGE_HOME}/openark-kiss/templates/configmap-assets.yaml" |
                yq "select(.metadata.name == \"assets-patches\") | .data.\"${name}\"" |
                sudo tee "./patches/${name}" >/dev/null
            sudo chmod 555 "./patches/${name}"
        done

        # Copy preset
        sudo cp -Lr "${IMAGE_HOME}/preset" './'

        cd - >/dev/null
    )

    # Update boot flags with cloud-init autoinstall
    sudo sed -i 's|---|autoinstall ds=nocloud\\\;s=/cdrom/autoinstall/ ---|g' './boot/grub/grub.cfg'
    sudo sed -i 's/^\(menuentry \)"Try or Install Ubuntu Server"/\1"Install SmartX K8S"/g' './boot/grub/grub.cfg'
    sudo sed -i 's/^\(menuentry \)"Ubuntu Server/\1"SmartX K8S/g' './boot/grub/grub.cfg'
    sudo sed -i 's/^\(set timeout=\)[0-9]*/\10/g' './boot/grub/grub.cfg'

    # Regenerate md5
    find '!' -name 'md5sum.txt' -follow -type f -exec "$(which md5sum)" {} \; | sudo tee ./md5sum.txt >/dev/null
    sudo ln -sf . "${OS_DIST}"
    ;;
*)
    echo "Unsupported installation kind: ${INSTALL_KIND}" >&2
    exec false
    ;;
esac

# Create a target ISO:
xorriso \
    -as mkisofs \
    -boot-info-table \
    -boot-load-size 4 \
    -c 'boot.catalog' \
    -e 'EFI/boot/bootx64.efi' \
    -isohybrid-gpt-basdat \
    -J \
    -l \
    -no-emul-boot \
    -o "${OS_TGT_FILE}" \
    -partition_offset '16' \
    -r \
    -V "SmartX K8S :: ${OS_DIST}-${OS_VERSION}" \
    '.' >&2 2>/dev/null

echo "* Completed building an ISO: ${OS_TGT_FILE}"

###########################################################
#   Cleanup                                               #
###########################################################

echo "* Cleaning up"
cd "${ROOT}"
exec sudo rm -rf "${IMAGE_HOME}" "${OS_SRC}"
