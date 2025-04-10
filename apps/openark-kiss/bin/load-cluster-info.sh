#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

# Verbose
set -x

# Load control planes
if [ "${#kiss_cluster_control_planes}" == '0' ]; then
    case "x${kiss_ansible_task_name}" in
    'xjoin')
        state='^(Ready|Running)$'
        ;;
    *)
        state='^Running$'
        ;;
    esac
    declare -a control_planes=($(
        kubectl get boxes.kiss.ulagbulag.io -o json |
            jq -r '[
            .items[]
                | select( .spec.group.clusterName == "'"${kiss_cluster_name}"'" )
                | select( .spec.group.role == "ControlPlane" )
                | select( .status.bindGroup.clusterName == "'"${kiss_cluster_name}"'" )
                | select( .status.bindGroup.role == "ControlPlane" )
                | select( .status.state | test("'"${state}"'") )
            ] | sort_by( .metadata.creationTimestamp, .metadata.name )[]
            | "kube_control_plane:" + .metadata.name + ":" + .status.access.primary.address
        '
    ))
    unset role state
else
    declare -a control_planes=(${kiss_cluster_control_planes})
fi

# Load ETCD nodes
if [ "${#kiss_cluster_etcd_nodes}" == '0' ]; then
    declare -a etcd_nodes=($(
        echo ${control_planes[@]:0:$(((($kiss_etcd_nodes_max + 1) / 2 - 1) * 2 + 1))} |
            sed 's/kube_control_plane:/etcd:/g'
    ))
else
    declare -a etcd_nodes=(${kiss_cluster_etcd_nodes})
fi

# Load worker nodes
if [ "${#kiss_cluster_worker_nodes}" == '0' ] && [ "x${kiss_ansible_task_name}" == 'xupgrade' ] && [ "x${kiss_group_role}" == 'xWorker' ]; then
    state='^Running$'
    declare -a worker_nodes=($(
        kubectl get boxes.kiss.ulagbulag.io -o json |
            jq -r '[
            .items[]
                | select( .spec.group.clusterName == "'"${kiss_cluster_name}"'" )
                | select( .spec.group.role != "ControlPlane" )
                | select( .status.bindGroup.clusterName == "'"${kiss_cluster_name}"'" )
                | select( .status.bindGroup.role != "ControlPlane" )
                | select( .status.state | test("'"${state}"'") )
            ] | sort_by( .metadata.creationTimestamp, .metadata.name )[]
            | "worker:" + .metadata.name + ":" + .status.access.primary.address
        '
    ))
    unset state
else
    declare -a worker_nodes=(${kiss_cluster_worker_nodes})
fi

# Get the target node name
case "x${kiss_ansible_task_name}" in
'xcommission' | 'xjoin' | 'xping')
    if [ "x${ansible_host}" == 'x' ]; then
        echo "Cannot infer ansible host in this task: ${kiss_ansible_task_name}"
        exit 1
    fi
    ;;
'xreset' | 'xupgrade')
    if [ "x${ansible_host}" != 'x' ]; then
        echo "Cannot set ansible host in this task: ${kiss_ansible_task_name}"
        exit 1
    fi
    ansible_host="$(
        echo "${control_planes[0]}" | cut -d ':' -f 2
    )"
    ;;
*)
    echo "Unknown task name: ${kiss_ansible_task_name}"
    exit 1
    ;;
esac

# Get the target node information

ansible_host_id="${ansible_host}"
ansible_host_uuid="$(
    kubectl get boxes.kiss.ulagbulag.io "${ansible_host}" \
        -o jsonpath \
        --template '{.spec.machine.uuid}'
)"
ansible_ssh_host="$(
    kubectl get boxes.kiss.ulagbulag.io "${ansible_host}" \
        -o jsonpath \
        --template '{.status.access.primary.address}'
)"

# Write down cluster information
echo ${control_planes[@]} >'./control_planes.txt'
echo ${etcd_nodes[@]} >'./etcd_nodes.txt'
echo ${worker_nodes[@]} >'./worker_nodes.txt'

# Write down node information
echo ${ansible_host} >'./ansible_host.txt'
echo ${ansible_host_id} >'./ansible_host_id.txt'
echo ${ansible_host_uuid} >'./ansible_host_uuid.txt'
echo ${ansible_ssh_host} >'./ansible_ssh_host.txt'
