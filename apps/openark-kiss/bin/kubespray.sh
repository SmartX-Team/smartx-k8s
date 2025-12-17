#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Execute a specific kubespray playbook in the cluster.

# Prehibit errors
set -e -o pipefail

function cleanup() {
    if [ -d "${WORKDIR}" ]; then
        cd - >/dev/null
        rm -rf "${WORKDIR}"
    fi
}

function terminate() {
    cleanup
    exec true
}

trap -- 'terminate' SIGINT
trap -- 'terminate' SIGTERM

###########################################################
#   Auto-detect Container Runtime                         #
###########################################################

if [ "x${CONTAINER_RUNTIME}" == 'x' ]; then
    if nerdctl ps >/dev/null 2>/dev/null; then
        CONTAINER_RUNTIME="nerdctl"
    elif podman ps >/dev/null 2>/dev/null; then
        CONTAINER_RUNTIME="podman"
    elif docker ps >/dev/null 2>/dev/null; then
        CONTAINER_RUNTIME="docker"
    else
        echo "Container runtime not found: nerdctl, docker, podman" >&2
        exit 1
    fi
fi

###########################################################
#   Main Function                                         #
###########################################################

# Define a main function
function main() {
    # Create a temporary directory
    if [ "x${WORKDIR}" == 'x' ] || [ ! -d "${WORKDIR}" ]; then
        export WORKDIR="$(mktemp -d)"
        chmod 700 "${WORKDIR}"
    fi
    cd "${WORKDIR}"

    (
        # Begin building kubespray inventory
        mkdir -p inventory
        cd inventory

        # Register bootstrapper node(s)
        {{- $nodeName := .Values.bootstrapper.node.name }}
        {{- $nodeIP := .Values.bootstrapper.network.address.ipv4 }}
        echo '{}' |
            yq '.all.hosts.{{ $nodeName }}.ansible_host = {{ $nodeIP | quote }}' |
            yq '.all.hosts.{{ $nodeName }}.ansible_host_key_checking = false' |
            yq '.all.hosts.{{ $nodeName }}.ansible_ssh_host = {{ $nodeIP | quote }}' |
            yq '.all.hosts.{{ $nodeName }}.ansible_ssh_port = 22' |
            yq '.all.hosts.{{ $nodeName }}.ansible_ssh_user = {{ .Values.kiss.auth.ssh.username | quote }}' |
            yq '.all.hosts.{{ $nodeName }}.ip = {{ $nodeIP | quote }}' |
            yq '.all.hosts.{{ $nodeName }}.name = {{ $nodeName | quote }}' |
            yq '.etcd.hosts.{{ $nodeName }} = {}' |
            yq '.kube_control_plane.hosts.{{ $nodeName }} = {}' |
            yq '.kube_node.hosts.{{ $nodeName }} = {}' |
            cat >./hosts.yaml
        unset node_name

        # Complete building kubespray inventory
        cd - >/dev/null
    )

    (
        # Begin building SSH inventory
        mkdir ssh
        chmod 700 ssh
        cd ssh

        # Get SSH private key
        echo -e {{ .Values.kiss.auth.ssh.key.private | quote }} >./key
        chmod 400 ./key

        # Get SSH public key
        echo {{ .Values.kiss.auth.ssh.key.public | quote }} >./key.pub

        # Complete building SSH inventory
        cd - >/dev/null
    )

    # Enable KISS tasks
    declare -a EXTRA_ENVS=()
    declare -a EXTRA_MOUNTS=()
    {{- $tasks := printf "${WORKDIR}/apps/%s/tasks" .Chart.Name }}
    if [ -d {{ $tasks | quote }} ]; then
        # Validate command
        OPENARK_KISS_TASK="$1"
        if [ "${OPENARK_KISS_TASK}" == 'x' ]; then
            echo 'Missing OpenARK KISS task' >&2
            exec false
        elif [ ! -d {{ printf "%s/${OPENARK_KISS_TASK}" $tasks | quote }} ]; then
            echo "Unknown OpenARK KISS task: ${OPENARK_KISS_TASK}" >&2
            exec false
        fi
        PLAYBOOK='/opt/playbook/playbook-control_plane.yaml'

        # Configure environment variables
        envFile="${WORKDIR}/.env"
        cat <<__EOF >"${envFile}"
ansible_host={{ $nodeName }}
ansible_host_id={{ $nodeName }}
ansible_host_uuid={{ $nodeName }}
ansible_ssh_host={{ $nodeIP }}
ansible_ssh_private_key_file=/root/.ssh/id_ed25519
ansible_user={{ .Values.kiss.auth.ssh.username }}
kiss_allow_critical_commands={{ .Values.kiss.commission.allowCriticalCommands }}
kiss_allow_pruning_network_interfaces={{ .Values.kiss.commission.allowPruningNetworkInterfaces }}
kiss_ansible_task_name=${OPENARK_KISS_TASK}
kiss_cluster_control_planes={{ printf "kube_control_plane:%s:%s" $nodeName $nodeIP }}
kiss_cluster_etcd_nodes={{ printf "etcd:%s:%s" $nodeName $nodeIP }}
kiss_cluster_name={{ .Values.cluster.group }}
kiss_cluster_name_snake_case={{ .Values.cluster.group | snakecase }}
kiss_cluster_domain={{ .Values.cluster.domainBase | snakecase }}
kiss_cluster_is_default={{ eq "default" .Values.cluster.group }}
kiss_cluster_is_new=true
kiss_cluster_worker_nodes={{ printf "kube_node:%s:%s" $nodeName $nodeIP }}
kiss_group_default_role={{ .Values.kiss.group.defaultRole }}
kiss_group_enable_default_cluster={{ .Values.kiss.group.enableDefaultCluster }}
kiss_group_force_reset={{ .Values.kiss.group.forceReset }}
kiss_group_force_reset_os={{ .Values.kiss.group.forceResetOS }}
kiss_group_reset_storage={{ .Values.kiss.group.resetStorage }}
kiss_group_role=ControlPlane
kiss_group_role_is_domain_specific=false
kiss_group_role_is_member=false
kiss_network_interface_mtu_size={{ .Values.network.interface.mtu }}
kiss_network_ipv4_dhcp_duration={{ .Values.network.ipv4.dhcp.duration }}
kiss_network_ipv4_dhcp_range_begin={{ .Values.network.ipv4.dhcp.range.begin }}
kiss_network_ipv4_dhcp_range_end={{ .Values.network.ipv4.dhcp.range.end }}
kiss_network_ipv4_gateway={{ .Values.network.ipv4.gateway }}
kiss_network_ipv4_subnet={{ .Values.network.ipv4.subnet }}
kiss_network_ipv4_subnet_address={{ index ( .Values.network.ipv4.subnet | split "/" ) "_0" }}
kiss_network_ipv4_subnet_mask={{ include "cidrToMask" ( index ( .Values.network.ipv4.subnet | split "/" ) "_1" ) }}
kiss_network_ipv4_subnet_mask_prefix={{ index ( .Values.network.ipv4.subnet | split "/" ) "_1" }}
kiss_network_nameserver_incluster_ipv4={{ .Values.cluster.nameservers.loadBalancer.ipv4 | default .Values.cluster.nameservers.incluster.ipv4 }}
kiss_network_wireless_wifi_key_mgmt={{ .Values.network.wireless.wifi.key.mgmt }}
kiss_network_wireless_wifi_key_psk={{ .Values.network.wireless.wifi.key.psk }}
kiss_network_wireless_wifi_ssid={{ .Values.network.wireless.wifi.ssid }}
kiss_os_default={{ printf "%s%s" .Values.kiss.os.dist ( .Values.kiss.os.version | replace "." "" ) }}
kiss_os_dist={{ .Values.kiss.os.dist }}
kiss_os_kernel={{ .Values.kiss.os.kernel }}
kiss_power_intel_amt_host=
kiss_power_intel_amt_username={{ .Values.kiss.power.intelAmt.username }}
kiss_power_intel_amt_password={{ .Values.kiss.power.intelAmt.password }}
kiss_power_ipmi_username={{ .Values.kiss.power.ipmi.username }}
kiss_power_ipmi_password={{ .Values.kiss.power.ipmi.password }}
__EOF
        chmod 400 "${envFile}"
        EXTRA_ENVS+=('--env-file' "${envFile}")

        # Configure mount path
        mkdir -p {{ printf "%s/common/tasks" $tasks | quote }}
        EXTRA_MOUNTS+=('--mount' "type=bind,source={{ printf "%s/common" $tasks | quote }},dst=/opt/playbook,readonly")
        EXTRA_MOUNTS+=('--mount' "type=bind,source={{ printf "%s/${OPENARK_KISS_TASK}" $tasks | quote }},dst=/opt/playbook/tasks,readonly")
    else
        PLAYBOOK="$1"
        if [ "${PLAYBOOK}" == 'x' ]; then
            echo 'Missing kubespray playbook' >&2
            exec false
        fi
    fi

    # Append privileged tag if nested container
    declare -a EXTRA_ARGS=()
    if grep -Posq '(docker|kubepods|lxc)' /proc/1/cgroup; then
        EXTRA_ARGS+=('--privileged')
    fi

    # Cleanup old container
    local container_name='smartx-k8s-bootstrap'
    nerdctl stop -t 5 "${container_name}" >/dev/null 2>/dev/null || true
    nerdctl rm -f "${container_name}" >/dev/null 2>/dev/null || true

    # Deploy a k8s cluster
    local container_id="$(
        ${CONTAINER_RUNTIME} run -d \
            ${EXTRA_ENVS[@]} \
            --init \
            --mount type=bind,source="${WORKDIR}/inventory/",dst=/inventory \
            --mount type=bind,source="${WORKDIR}/ssh/",dst=/root/.ssh,readonly \
            ${EXTRA_MOUNTS[@]} \
            --name "${container_name}" \
            --net host \
            --security-opt seccomp='unconfined' \
            ${EXTRA_ARGS[@]} \
            {{ printf "%s:%s" .Values.kiss.image.repo .Values.kiss.image.tag | quote }} \
            ansible-playbook \
            --become \
            --become-user 'root' \
            --inventory '/inventory/defaults.yaml' \
            --inventory '/inventory/hosts.yaml' \
            --private-key '/root/.ssh/key' \
            "${PLAYBOOK}" ${@:2}
    )"

    # Wait until the container has been completed
    until [ "$(${CONTAINER_RUNTIME} inspect "${container_id}" 2>/dev/null | yq '.0.State.Running')" == 'false' ]; do
        ${CONTAINER_RUNTIME} logs -f "${container_id}" || true
        until ${CONTAINER_RUNTIME} inspect "${container_id}" >/dev/null 2>/dev/null; do
            sleep 1
        done
    done

    # Terminate the container
    local exit_code="$(${CONTAINER_RUNTIME} inspect "${container_id}" 2>/dev/null | yq '.0.State.ExitCode')"
    ${CONTAINER_RUNTIME} rm -f "${container_id}" >/dev/null 2>/dev/null || true

    # Cleanup
    cleanup
    exit "${exit_code}"
}

# Execute main function
main $@
