#!/usr/bin/env bash

set -euo pipefail

export LC_ALL=C
export LANG=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO_ROOT="$(cd "${ROOT}/../.." && pwd)"

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

disabled="$(mktemp)"
enabled="$(mktemp)"
cross_namespace="$(mktemp)"
applications="$(mktemp)"
frozen_applications="$(mktemp)"

trap '
    rm -f \
        "${disabled}" \
        "${enabled}" \
        "${cross_namespace}" \
        "${applications}" \
        "${frozen_applications}"
' EXIT

helm lint \
    "${ROOT}" \
    --set enabled=true \
    >/dev/null

printf 'PASS: Helm lint with provisioning enabled\n'

helm template \
    rook-ceph-provisioning \
    "${ROOT}" \
    --namespace csi-rook-ceph-provisioning \
    >"${disabled}"

helm template \
    rook-ceph-provisioning \
    "${ROOT}" \
    --namespace csi-rook-ceph-provisioning \
    --set enabled=true \
    >"${enabled}"

helm template \
    rook-ceph-provisioning \
    "${ROOT}" \
    --namespace csi-rook-ceph-provisioning \
    --set enabled=true \
    --set rookNamespace=rook-discovery \
    >"${cross_namespace}"

helm template \
    smartx \
    "${REPO_ROOT}" \
    --set rookCeph.provisioning.manageStorageNodes=true \
    --set rookCeph.provisioning.enabled=true \
    >"${applications}"

helm template \
    smartx \
    "${REPO_ROOT}" \
    --set rookCeph.provisioning.manageStorageNodes=true \
    >"${frozen_applications}"

if helm template \
    smartx \
    "${REPO_ROOT}" \
    --set rookCeph.provisioning.enabled=true \
    >/dev/null 2>&1; then
    fail 'provisioning was enabled without selecting its feature'
fi

printf 'PASS: provisioning requires explicit storage ownership\n'

if helm template \
    smartx \
    "${REPO_ROOT}" \
    --set rookCeph.provisioning.manageStorageNodes=true \
    --set rookCeph.provisioning.enabled=true \
    --set-json 'rookCeph.cluster.cephClusterSpec.storage.nodes=[{"name":"manual"}]' \
    >/dev/null 2>&1; then
    fail 'runtime-owned storage.nodes accepted a declarative node'
fi

printf 'PASS: declarative storage nodes are rejected\n'

if helm template \
    smartx \
    "${REPO_ROOT}" \
    --set rookCeph.provisioning.manageStorageNodes=true \
    --set rookCeph.cluster.cephClusterSpec.storage.deviceFilter='sd.*' \
    >/dev/null 2>&1; then
    fail 'runtime-owned storage accepted a cluster-level device selector'
fi

printf 'PASS: cluster-level storage selectors are rejected\n'

if grep -Eq '^[[:space:]]*kind:' "${disabled}"; then
    fail 'disabled chart rendered Kubernetes resources'
fi

printf 'PASS: disabled chart renders no runtime resources\n'

python3 \
    - "${enabled}" \
    "${cross_namespace}" \
    "${applications}" \
    "${frozen_applications}" <<'PY'
from __future__ import annotations

import re
import sys
from collections import Counter
from pathlib import Path


enabled = Path(sys.argv[1]).read_text(encoding="utf-8")
cross_namespace = Path(sys.argv[2]).read_text(encoding="utf-8")
applications = Path(sys.argv[3]).read_text(encoding="utf-8")
frozen_applications = Path(sys.argv[4]).read_text(encoding="utf-8")


def documents(content: str) -> list[str]:
    return [
        document.strip()
        for document in re.split(r"(?m)^---\s*$", content)
        if re.search(r"(?m)^kind:\s*\S+\s*$", document)
    ]


def kind(document: str) -> str:
    match = re.search(
        r"(?m)^kind:\s*(\S+)\s*$",
        document,
    )

    if match is None:
        raise SystemExit("FAIL: rendered document has no kind")

    return match.group(1)


def only_kind(content: str, target: str) -> str:
    matches = [
        document
        for document in documents(content)
        if kind(document) == target
    ]

    if len(matches) != 1:
        raise SystemExit(
            f"FAIL: expected one {target}, found {len(matches)}"
        )

    return matches[0]


def require(
    document: str,
    value: str,
    description: str,
) -> None:
    if value not in document:
        raise SystemExit(
            f"FAIL: missing {description}: {value!r}"
        )


enabled_documents = documents(enabled)
counts = Counter(kind(document) for document in enabled_documents)

expected_counts = {
    "ServiceAccount": 2,
    "Role": 3,
    "RoleBinding": 3,
    "ClusterRole": 1,
    "ClusterRoleBinding": 1,
    "DaemonSet": 1,
    "Deployment": 1,
}

for resource_kind, expected in expected_counts.items():
    actual = counts[resource_kind]

    if actual != expected:
        raise SystemExit(
            f"FAIL: expected {expected} {resource_kind} "
            f"resource(s), found {actual}"
        )

print("PASS: expected runtime resource set")

if counts["ConfigMap"] != 0:
    raise SystemExit("FAIL: runtime code must not be loaded from a ConfigMap")

print("PASS: runtime code is baked into the image")

daemonset = only_kind(enabled, "DaemonSet")

for value, description in (
    (
        '"node-role.kubernetes.io/kiss": "Storage"',
        "Storage node selector",
    ),
    (
        "path: /sys",
        "host sysfs mount",
    ),
    (
        "path: /dev",
        "host device mount",
    ),
    (
        "mountPath: /host/sys",
        "read-only host sysfs volume mount",
    ),
    (
        "mountPath: /host/dev",
        "read-only host device volume mount",
    ),
    (
        "runAsUser: 0",
        "root agent UID for signature probing",
    ),
    (
        "privileged: true",
        "block-device read access",
    ),
    (
        "readOnlyRootFilesystem: true",
        "agent read-only root filesystem",
    ),
    (
        "/usr/local/bin/openark-rook-ceph-agent",
        "compiled Rust agent",
    ),
):
    require(daemonset, value, description)

if daemonset.count("readOnly: true") < 2:
    raise SystemExit(
        "FAIL: agent host volume mounts are not read-only"
    )

print("PASS: Storage-label node agent DaemonSet")

deployment = only_kind(enabled, "Deployment")

for value, description in (
    (
        "replicas: 1",
        "single controller replica",
    ),
    (
        "type: Recreate",
        "single-writer Recreate strategy",
    ),
    (
        "name: DEVICE_CLASS_MAP",
        "device-class mapping environment",
    ),
    (
        "name: INVENTORY_MAX_AGE_SECONDS",
        "inventory age boundary",
    ),
    (
        "name: MINIMUM_DEVICE_BYTES",
        "minimum device size",
    ),
    (
        "/usr/local/bin/openark-rook-ceph-controller",
        "compiled Rust controller",
    ),
    (
        "runAsUser: 2000",
        "non-root controller UID",
    ),
    (
        "readOnlyRootFilesystem: true",
        "controller read-only root filesystem",
    ),
):
    require(deployment, value, description)

print("PASS: single-writer controller Deployment")

all_enabled = "\n".join(enabled_documents)

for value, description in (
    (
        "resources:\n      - cephclusters",
        "CephCluster RBAC",
    ),
    (
        "resources:\n      - nodes",
        "Node RBAC",
    ),
    (
        "resources:\n      - configmaps",
        "ConfigMap RBAC",
    ),
    (
        "- patch",
        "CephCluster patch permission",
    ),
):
    require(all_enabled, value, description)

print("PASS: runtime RBAC")

rook_roles = [
    document
    for document in documents(cross_namespace)
    if (
        kind(document) in {"Role", "RoleBinding"}
        and "app.kubernetes.io/component: controller-rook"
        in document
    )
]

if len(rook_roles) != 2:
    raise SystemExit(
        "FAIL: expected Rook Role and RoleBinding"
    )

for document in rook_roles:
    require(
        document,
        'namespace: "rook-discovery"',
        "cross-namespace Rook RBAC namespace",
    )

require(
    all_enabled,
    'resourceNames:\n      - "rook-ceph-cluster"',
    "named CephCluster RBAC",
)

print("PASS: cross-namespace Rook RBAC")

rook_application = next(
    document
    for document in documents(applications)
    if 'name: "smartx-rook-ceph-cluster"' in document
)

for value, description in (
    (
        "- /spec/storage/nodes",
        "runtime-owned Ceph storage inventory",
    ),
    (
        "useAllDevices: false",
        "explicit device selection invariant",
    ),
    (
        "useAllNodes: false",
        "explicit node selection invariant",
    ),
    (
        "nodes: []",
        "empty initial runtime inventory",
    ),
):
    require(rook_application, value, description)

if "$cluster/patches/rook-ceph-cluster/values.yaml" in rook_application:
    raise SystemExit("FAIL: Rook cluster has multiple child policy channels")

provisioning_application = next(
    document
    for document in documents(applications)
    if 'name: "smartx-rook-ceph-provisioning"' in document
)

for value, description in (
    (
        'namespace: "csi-rook-ceph-provisioning"',
        "isolated provisioning namespace",
    ),
    (
        "prune: true",
        "runtime resource pruning",
    ),
    (
        "resources-finalizer.argocd.argoproj.io",
        "runtime application finalizer",
    ),
):
    require(provisioning_application, value, description)

print("PASS: GitOps ownership and lifecycle")

frozen_rook_application = next(
    document
    for document in documents(frozen_applications)
    if 'name: "smartx-rook-ceph-cluster"' in document
)

for value, description in (
    (
        "- /spec/storage/nodes",
        "retained runtime inventory ownership",
    ),
    (
        "useAllDevices: false",
        "frozen explicit device selection",
    ),
    (
        "useAllNodes: false",
        "frozen explicit node selection",
    ),
):
    require(frozen_rook_application, value, description)

print("PASS: disabled runtime retains fail-closed storage ownership")
PY

printf 'All Helm render tests passed.\n'
