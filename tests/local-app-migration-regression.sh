#!/usr/bin/env bash

set -euo pipefail

EECS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK_DIR="$(dirname "$EECS_DIR")"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

require() {
  command -v "$1" >/dev/null || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

for command in helm yq grep; do
  require "$command"
done

render_apps() {
  helm template cilium-lb-ipam "$EECS_DIR/apps/cilium-lb-ipam" \
    --namespace kube-system \
    -f "$WORK_DIR/b-k8s/patches/cilium-lb-ipam/values.yaml" >"$TMP_DIR/b-cilium.yaml"
  helm template cilium-lb-ipam "$EECS_DIR/apps/cilium-lb-ipam" \
    --namespace kube-system \
    -f "$WORK_DIR/c-k8s/patches/cilium-lb-ipam/values.yaml" >"$TMP_DIR/c-cilium.yaml"

  helm template workload-namespace "$EECS_DIR/apps/workload-namespace" \
    --namespace default \
    -f "$WORK_DIR/b-k8s/patches/workload-namespace/values.yaml" >"$TMP_DIR/b-namespace.yaml"
  helm template workload-namespace "$EECS_DIR/apps/workload-namespace" \
    --namespace default \
    -f "$WORK_DIR/c-k8s/patches/workload-namespace/values.yaml" >"$TMP_DIR/c-namespace.yaml"

  helm template rook-ceph-provisioning "$EECS_DIR/apps/rook-ceph-provisioning" \
    --namespace csi-rook-ceph \
    -f "$WORK_DIR/b-k8s/patches/rook-ceph-provisioning/values.yaml" >"$TMP_DIR/b-rook.yaml"
  helm template rook-ceph-provisioning "$EECS_DIR/apps/rook-ceph-provisioning" \
    --namespace csi-rook-ceph \
    -f "$WORK_DIR/c-k8s/patches/rook-ceph-provisioning/values.yaml" >"$TMP_DIR/c-rook.yaml"

  helm template rook-ceph-rgw "$EECS_DIR/apps/rook-ceph-rgw" \
    --namespace csi-rook-ceph \
    -f "$WORK_DIR/b-k8s/patches/rook-ceph-rgw/values.yaml" >"$TMP_DIR/b-rgw.yaml"

  helm template karmada-members "$EECS_DIR/apps/karmada-members" \
    --namespace karmada-system \
    -f "$WORK_DIR/tower-k8s/patches/karmada-members/values.yaml" >"$TMP_DIR/karmada-members.yaml"

  helm template remote-gitops "$EECS_DIR/apps/remote-gitops" \
    --namespace argo \
    -f "$WORK_DIR/tower-k8s/patches/remote-gitops/values.yaml" >"$TMP_DIR/remote-gitops.yaml"
}

assert_yq() {
  local expression="$1"
  local file="$2"
  yq -e "$expression" "$file" >/dev/null
}

assert_pool() {
  local file="$1"
  local name="$2"
  local start="$3"
  local stop="$4"
  local policy="$5"

  assert_yq "select(.kind == \"CiliumLoadBalancerIPPool\" and .metadata.name == \"$name\") |
    .spec.blocks[0].start == \"$start\" and
    .spec.blocks[0].stop == \"$stop\"" "$file"
  assert_yq "select(.kind == \"CiliumL2AnnouncementPolicy\" and .metadata.name == \"$policy\") |
    .spec.loadBalancerIPs == true" "$file"
}

assert_rook() {
  local file="$1"

  assert_yq 'select(.kind == "Job" and .metadata.name == "rook-ceph-poc-config") |
    .metadata.namespace == "csi-rook-ceph" and
    .spec.template.spec.serviceAccountName == "rook-ceph-default" and
    .spec.template.spec.containers[0].image == "quay.io/ceph/ceph:v19.2.1" and
    .spec.activeDeadlineSeconds == 1800 and
    .spec.backoffLimit == 20' "$file"

  yq -r 'select(.kind == "Job" and .metadata.name == "rook-ceph-poc-config") |
    .spec.template.spec.containers[0].command[]' "$file" |
    grep -F 'ceph osd pool set .mgr size 1' >/dev/null
  yq -r 'select(.kind == "Job" and .metadata.name == "rook-ceph-poc-config") |
    .spec.template.spec.containers[0].command[]' "$file" |
    grep -F 'ceph osd pool set .mgr min_size 1' >/dev/null
}

render_apps

assert_pool "$TMP_DIR/b-cilium.yaml" b-lb-pool 10.33.142.1 10.33.142.254 b-l2-policy
assert_pool "$TMP_DIR/c-cilium.yaml" c-lb-pool 10.33.143.1 10.33.143.254 c-l2-policy

for cluster in b c; do
  CLUSTER="$cluster" assert_yq 'select(.kind == "Namespace" and
    .metadata.name == "scalex-rgw-analysis-web") |
    .metadata.labels."scalex.io/infra-owner" == strenv(CLUSTER) + "-k8s" and
    .metadata.annotations."argocd.argoproj.io/sync-options" == "Delete=false,Prune=false"' \
    "$TMP_DIR/$cluster-namespace.yaml"
done

assert_rook "$TMP_DIR/b-rook.yaml"
assert_rook "$TMP_DIR/c-rook.yaml"

assert_yq 'select(.kind == "ObjectBucketClaim" and .metadata.name == "tower-harbor-registry") |
  .metadata.namespace == "csi-rook-ceph" and
  .metadata.labels."scalex.io/infra-owner" == "b-k8s" and
  .spec.bucketName == "tower-harbor-registry" and
  .spec.storageClassName == "ceph-bucket"' "$TMP_DIR/b-rgw.yaml"
assert_yq 'select(.kind == "ObjectBucketClaim" and .metadata.name == "rgw-analysis-web-bucket") |
  .metadata.namespace == "scalex-rgw-analysis-web" and
  .metadata.annotations."scalex.io/bucket-naming-mode" == "compatibility-fixed" and
  .metadata.labels."scalex.io/infra-owner" == "b-k8s" and
  .spec.bucketName == "rgw-analysis-web-poc" and
  .spec.storageClassName == "ceph-bucket"' "$TMP_DIR/b-rgw.yaml"
[[ "$(yq -r 'select(.kind == "ObjectBucketClaim") | .metadata.name' \
  "$TMP_DIR/b-rgw.yaml" | grep -cv '^---$')" -eq 2 ]]
assert_yq 'select(.kind == "Service" and .metadata.name == "scalex-poc-rgw") |
  .metadata.annotations."lbipam.cilium.io/ips" == "10.33.142.10" and
  .spec.type == "LoadBalancer" and
  .spec.selector.rook_object_store == "scalex-poc" and
  .spec.ports[0].port == 80 and .spec.ports[0].targetPort == 8080' "$TMP_DIR/b-rgw.yaml"

assert_yq 'select(.kind == "Job" and .metadata.name == "karmada-member-join") |
  .spec.backoffLimit == 1' "$TMP_DIR/karmada-members.yaml"
member_script="$(yq -r 'select(.kind == "ConfigMap" and .metadata.name == "karmada-member-join-script") |
  .data."join.sh"' "$TMP_DIR/karmada-members.yaml")"
grep -F 'join_member "b" "argo" "cluster-b"' <<<"$member_script" >/dev/null
grep -F 'join_member "c" "argo" "cluster-c"' <<<"$member_script" >/dev/null
grep -F 'cluster-karmada' <<<"$(cat "$TMP_DIR/karmada-members.yaml")" >/dev/null

if [[ -f "$TMP_DIR/remote-gitops.yaml" ]]; then
  assert_yq 'select(.kind == "Application" and .metadata.name == "b") |
    .spec.destination.name == "tower" and
    .spec.sources[0].repoURL == "https://github.com/SJoon99/eecs-k8s.git" and
    .spec.sources[0].helm.valuesObject.applications.root.enabled == false and
    .spec.sources[1].repoURL == "https://github.com/SmartX-Team/b-k8s.git" and
    .spec.sources[1].targetRevision == "main"' "$TMP_DIR/remote-gitops.yaml"
  assert_yq 'select(.kind == "Application" and .metadata.name == "c") |
    .spec.destination.name == "tower" and
    .spec.sources[0].helm.valuesObject.applications.root.enabled == false and
    .spec.sources[1].repoURL == "https://github.com/SmartX-Team/c-k8s.git" and
    .spec.sources[1].targetRevision == "main"' "$TMP_DIR/remote-gitops.yaml"
  assert_yq 'select(.kind == "Application" and .metadata.name == "tower-scalex-federation") |
    .spec.source.repoURL == "https://github.com/SmartX-Team/sandbox-scalex-federation.git" and
    .spec.source.path == "argocd"' "$TMP_DIR/remote-gitops.yaml"
fi

echo "local app migration regression: PASS"
