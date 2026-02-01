#!/usr/bin/env bash
# Copyright (c) 2025-2026 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
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

# Merge cluster values
export PRESET_URL="${BASEDIR}/preset"
values_file="${PRESET_URL}/values.yaml"
yq eval-all '. as $item ireduce ({}; . * $item )' \
    "${BASEDIR}/values.yaml" \
    "${values_file}" \
    >"${values_file}.patched"

# Collect node information
cat "${values_file}.patched" |
    yq ".bootstrapper.network.address.ipv4=\"${NODE_IP}\"" |
    yq ".bootstrapper.node.name=\"${NODE_ID}\"" |
    yq ".kiss.auth.ssh.key.private=\"$(cat "${USER_KEY_FILE}")\"" |
    yq ".kiss.auth.ssh.key.public=\"$(cat "${USER_KEY_FILE}.pub" | awk '{print $1 " " $2}')\"" |
    yq ".network.interface.mtu=${NETDEV_MTU}" |
    yq ".network.ipv4.dhcp.range.begin=\"${NODE_IP}\"" |
    yq ".network.ipv4.dhcp.range.end=\"${NODE_IP}\"" |
    yq ".network.ipv4.gateway=\"${NETDEV_GW}\"" |
    yq ".network.ipv4.subnet=\"${NODE_IP_NETWORK}\"" |
    cat >"${values_file}"

# Bootstrap a cluster
./hack/kubespray.sh 'commission'
./hack/kubespray.sh 'join'

# Build applications
apps_file="${BASEDIR}/apps.yaml"
helm template smartx \
    --values "${values_file}" \
    "${BASEDIR}" >"${apps_file}"

# Disable app-of-apps pattern
cluster_name="$(cat "${values_file}" | yq -r '.cluster.name')"
yq -i "select(.metadata.name != \"${cluster_name}\")" "${apps_file}"

# Enable digital twin
kubectl create namespace "$(cat "${values_file}" | yq -r '.twin.namespace')" || true

# Install CNI
cni_name='cilium'
cni_namespace="$(cat "${BASEDIR}/apps/${cni_name}/manifest.yaml" | yq -r '.spec.app.namespace')"
(
    cni_file="${BASEDIR}/${cni_name}.yaml"
    cat "${apps_file}" | yq "select(.metadata.name == \"*-${cni_name}\") | .spec.sources.0.helm.valuesObject" >"${cni_file}"
    kubectl create namespace "${cni_namespace}" || true

    helm repo add "${cni_name}" "$(cat "${BASEDIR}/apps/${cni_name}/manifest.yaml" | yq -r '.spec.source.repoUrl')"
    helm template "${cni_name}" "${cni_name}/${cni_name}" \
        --namespace "${cni_namespace}" \
        --values "${BASEDIR}/apps/${cni_name}/values.yaml" \
        --values "${cni_file}" \
        --version "$(cat "${BASEDIR}/apps/${cni_name}/manifest.yaml" | yq -r '.spec.source.version')" |
        kubectl apply -f - --server-side=true

    kubectl -n "${cni_namespace}" rollout status \
        "daemonset/${cni_name}" \
        "deployment/${cni_name}-operator"
    kubectl -n 'kube-system' rollout status 'deployment/coredns' || sleep 3
)

# Install CSR Approver
csr_name='kubelet-csr-approver'
csr_namespace="$(cat "${BASEDIR}/apps/${csr_name}/manifest.yaml" | yq -r '.spec.app.namespace')"
(
    csr_file="${BASEDIR}/${csr_name}.yaml"
    cat "${apps_file}" | yq "select(.metadata.name == \"*-${csr_name}\") | .spec.sources.0.helm.valuesObject" >"${csr_file}"
    kubectl create namespace "${csr_namespace}" || true

    helm repo add "${csr_name}" "$(cat "${BASEDIR}/apps/${csr_name}/manifest.yaml" | yq -r '.spec.source.repoUrl')"
    helm template "${csr_name}" "${csr_name}/${csr_name}" \
        --namespace "${csr_namespace}" \
        --values "${BASEDIR}/apps/${csr_name}/values.yaml" \
        --values "${csr_file}" \
        --version "$(cat "${BASEDIR}/apps/${csr_name}/manifest.yaml" | yq -r '.spec.source.version')" |
        kubectl apply -f - --server-side=true

    kubectl -n "${csr_namespace}" rollout status \
        "deployment/${csr_name}"
)

# Patch tolerations for builtins
(
    declare -a builtin_workloads=(
        'cert-manager/cert-manager'
        'cert-manager/cert-manager-cainjector'
        'cert-manager/cert-manager-webhook'
        'kube-system/coredns'
    )

    for workload in ${builtin_workloads[@]}; do
        workload_namespace="$(echo "${workload}" | grep -Po '^[0-9a-z-]+')"
        workload_name="$(echo "${workload}" | grep -Po '[0-9a-z-]+$')"
        kubectl patch --namespace "${workload_namespace}" deployment "${workload_name}" \
            -p '{"spec":{"template":{"spec":{"tolerations":[{"operator":"Exists"}]}}}}'
    done
)

# Install Argo CD
argocd_name='argo-cd'
argo_namespace="$(cat "${BASEDIR}/apps/${argocd_name}/manifest.yaml" | yq -r '.spec.app.namespace')"
(
    argocd_file="${BASEDIR}/${argocd_name}.yaml"
    cat "${apps_file}" | yq "select(.metadata.name == \"*-${argocd_name}\") | .spec.sources.0.helm.valuesObject" >"${argocd_file}"
    kubectl create namespace "${argo_namespace}" || true

    helm repo add "${argocd_name}" "$(cat "${BASEDIR}/apps/${argocd_name}/manifest.yaml" | yq -r '.spec.source.repoUrl')"
    helm template "${argocd_name}" "${argocd_name}/${argocd_name}" \
        --namespace "${argo_namespace}" \
        --values "${BASEDIR}/apps/${argocd_name}/values.yaml" \
        --values "${argocd_file}" \
        --version "$(cat "${BASEDIR}/apps/${argocd_name}/manifest.yaml" | yq -r '.spec.source.version')" |
        kubectl apply -f - --server-side=true

    kubectl -n "${argo_namespace}" rollout status \
        "deployment/${argocd_name}-argocd-application-controller" \
        "deployment/${argocd_name}-argocd-applicationset-controller" \
        "deployment/${argocd_name}-argocd-notifications-controller" \
        "deployment/${argocd_name}-argocd-redis" \
        "deployment/${argocd_name}-argocd-repo-server" \
        "deployment/${argocd_name}-argocd-server"
    sleep 3
)

# Install Argo Workflow
argowf_name='argo-workflows'
argowf_file="${BASEDIR}/${argowf_name}.yaml"
(
    cat "${apps_file}" | yq "select(.metadata.name == \"*-${argowf_name}\") | .spec.sources.0.helm.valuesObject" >"${argowf_file}"

    helm repo add "${argowf_name}" "$(cat "${BASEDIR}/apps/${argowf_name}/manifest.yaml" | yq -r '.spec.source.repoUrl')"
    helm template "${argowf_name}" "${argowf_name}/${argowf_name}" \
        --namespace "${argo_namespace}" \
        --values "${BASEDIR}/apps/${argowf_name}/values.yaml" \
        --values "${argowf_file}" \
        --version "$(cat "${BASEDIR}/apps/${argowf_name}/manifest.yaml" | yq -r '.spec.source.version')" |
        kubectl apply -f - --server-side=true
    helm template "${argowf_name}" "${BASEDIR}/apps/${argowf_name}" \
        --namespace "${argo_namespace}" \
        --values "${argowf_file}" \
        --version "$(cat "${BASEDIR}/apps/${argowf_name}/manifest.yaml" | yq -r '.spec.source.version')" |
        kubectl apply -f - --server-side=true

    kubectl -n "${argo_namespace}" rollout status \
        "deployment/${argowf_name}-server" \
        "deployment/${argowf_name}-workflow-controller"
    sleep 3
)

# Install Argo CD CLI
(
    curl -sSL -o /usr/local/bin/argocd "https://github.com/argoproj/argo-cd/releases/latest/download/argocd-linux-$(uname -m | sed 's/^x86_64/amd64/')"
    chmod 555 /usr/local/bin/argocd
)

# Install Argo CD Profiles
(
    # Update server IP
    node_ip="$(cat "${values_file}" | yq -r '.bootstrapper.network.address.ipv4')"
    yq -i ".clusters.0.cluster.server = \"https://${node_ip}:6443\"" ~/.kube/config

    # Connect to incluster Argo CD
    kubectl config rename-context 'kubernetes-admin@ops.openark' "${cluster_name}" || true
    kubectl config use-context "${cluster_name}"
    kubectl config set-context "${cluster_name}" --namespace="${argo_namespace}"
    argocd login --core

    # Add cluster
    argocd cluster add "${cluster_name}" --yes

    # Add appprojects
    kubectl -n argo get appprojects.argoproj.io default -o yaml |
        yq ".metadata.name = \"${cluster_name}-ops\"" |
        yq -r 'del(.metadata.creationTimestamp)' |
        yq -r 'del(.metadata.generation)' |
        yq -r 'del(.metadata.resourceVersion)' |
        yq -r 'del(.metadata.uid)' |
        kubectl apply -f - --server-side=true
)

# Install applications
kubectl -n "${argo_namespace}" apply -f "${apps_file}" --server-side=true

# Sync builtin applications
(
    argocd app sync "${cluster_name}-${cni_name}" || true
    argocd app sync "${cluster_name}-${csr_name}" || true
    argocd app sync "${cluster_name}-${argocd_name}" || true
    argocd app sync "${cluster_name}-${argowf_name}" || true
)

# Mark the bootstrapped node as "standalone"
(
    node_id="$(cat "${values_file}" | yq -r '.bootstrapper.node.name')"
    kubectl label nodes "${node_id}" --overwrite node-role.kubernetes.io/standalone='true'
)

# Cleanup
exec systemctl disable 'smartx-k8s-bootstrap.service'
