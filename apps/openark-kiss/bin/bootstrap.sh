#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Kubernetes Cluster Configuration
# Bootstrap a cluster

# Prehibit errors
set -e -o pipefail

# Configure environment variables
BASEDIR='/opt/smartx-k8s'
cd "${BASEDIR}"

## Detect primary network interface
NETDEV_ROUTE="$(ip route show | grep -P '^default via ' | head -n1)"
NETDEV="$(echo "${NETDEV_ROUTE}" | grep -Po '^default via [0-9\.]+ dev \K[0-9a-z]+')"
if [ "x${NETDEV}" == 'x' ]; then
    echo '' >&2
    exec false
fi
NETDEV_MTU="$(ip addr show dev "${NETDEV}" | grep -Po ' mtu \K[0-9]+')"
NETDEV_GW="$(echo "${NETDEV_ROUTE}" | grep -Po '^default via \K[0-9\.]+')"

NODE_ID="$(cat /sys/class/dmi/id/product_uuid)"
NODE_IP="$(ip addr show dev "${NETDEV}" | grep -Po 'inet \K[0-9\.]+' | head -n1)"
NODE_IP_SUBNET="$(ip addr show dev "${NETDEV}" | grep -Po 'inet \K[0-9\.]+/[0-9]+' | head -n1)"
NODE_IP_NETWORK="$(ipcalc "${NODE_IP_SUBNET}" | grep -Po '^Network: +\K[0-9\.]+/[0-9]+')"

USER_NAME={{ .Values.kiss.auth.ssh.username | quote }}
USER_KEY_TYPE='ed25519'
USER_KEY_FILE="/home/${USER_NAME}/.ssh/id_${USER_KEY_TYPE}"

# Generate a SSH key pair
if [ ! -f "${USER_KEY_FILE}" ]; then
    sudo -u "${USER_NAME}" ssh-keygen -q -t "${USER_KEY_TYPE}" -f "${USER_KEY_FILE}" -N ''
    cat "${USER_KEY_FILE}.pub" >"$(dirname "${USER_KEY_FILE}")/authorized_keys"
fi

# Collect node information
export PRESET_URL="${BASEDIR}/preset"
VALUES_FILE="${PRESET_URL}/values.yaml"
cat "${VALUES_FILE}" |
    yq ".bootstrapper.network.address.ipv4=\"${NODE_IP}\"" |
    yq ".bootstrapper.node.name=\"${NODE_ID}\"" "${VALUES_FILE}" |
    yq ".kiss.auth.ssh.key.private=\"$(cat "${USER_KEY_FILE}")\"" |
    yq ".kiss.auth.ssh.key.public=\"$(cat "${USER_KEY_FILE}.pub" | awk '{print $1 " " $2}')\"" |
    yq ".network.interface.mtu=${NETDEV_MTU}" |
    yq ".network.ipv4.dhcp.range.begin=\"${NODE_IP}\"" |
    yq ".network.ipv4.dhcp.range.end=\"${NODE_IP}\"" |
    yq ".network.ipv4.gateway=\"${NETDEV_GW}\"" |
    yq ".network.ipv4.subnet=\"${NODE_IP_NETWORK}\"" |
    cat >"${VALUES_FILE}.patched"
mv "${VALUES_FILE}.patched" "${VALUES_FILE}"

# Bootstrap a cluster
./hack/kubespray.sh 'commission'
./hack/kubespray.sh 'join'

# Cleanup
exec systemctl disable 'smartx-k8s-bootstrap.service'
