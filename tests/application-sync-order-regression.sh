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

for command in helm yq; do
  require "$command"
done

render_cluster() {
  local cluster="$1"

  helm template smartx "$EECS_DIR" \
    -f "$WORK_DIR/${cluster}-k8s/values.yaml" \
    >"$TMP_DIR/${cluster}.yaml" \
    2>"$TMP_DIR/${cluster}.stderr"
}

render_child_only() {
  local cluster="$1"

  helm template smartx "$EECS_DIR" \
    -f "$WORK_DIR/${cluster}-k8s/values.yaml" \
    --set applications.root.enabled=false \
    >"$TMP_DIR/${cluster}-child-only.yaml" \
    2>"$TMP_DIR/${cluster}-child-only.stderr"
}

assert_wave() {
  local cluster="$1"
  local application="$2"
  local wave="$3"

  yq -e "select(.kind == \"Application\" and .metadata.name == \"$application\") |
    .metadata.annotations.\"argocd.argoproj.io/sync-wave\" == \"$wave\"" \
    "$TMP_DIR/${cluster}.yaml" >/dev/null
}

render_cluster b
render_cluster c
render_cluster tower
render_child_only b
render_child_only c

assert_wave b b-cilium-lb-ipam 20
assert_wave b b-workload-namespace 10
assert_wave b b-rook-ceph-provisioning 50
assert_wave b b-rook-ceph-rgw 60
assert_wave c c-cilium-lb-ipam 20
assert_wave c c-workload-namespace 10
assert_wave c c-rook-ceph-provisioning 50
assert_wave tower tower-karmada-members 30
assert_wave tower tower-remote-gitops 40

for cluster in b c; do
  CLUSTER="$cluster" yq -e 'select(.kind == "Application" and
    .metadata.name == strenv(CLUSTER)) |
    .spec.sources[-1].targetRevision == "main" and
    .spec.sources[-1].repoURL == "https://github.com/SmartX-Team/" + strenv(CLUSTER) + "-k8s.git" and
    .spec.sources[-1].ref == "cluster"' "$TMP_DIR/$cluster.yaml" >/dev/null
done

yq -e 'select(.kind == "Application" and .metadata.name == "tower") |
  .spec.sources[-1].targetRevision == "ops" and
  .spec.sources[-1].repoURL == "https://github.com/SmartX-Team/sandbox-tower-k8s.git" and
  .spec.sources[-1].ref == "cluster"' "$TMP_DIR/tower.yaml" >/dev/null

yq -e 'select(.kind == "Application" and .metadata.name == "tower-harbor") |
  .spec.destination.name == "tower" and
  .spec.destination.namespace == "harbor" and
  .spec.sources[0].helm.valuesObject.externalURL == "http://10.34.25.18" and
  .spec.sources[0].helm.valueFiles[1] == "$cluster/patches/harbor/values.yaml"' \
  "$TMP_DIR/tower.yaml" >/dev/null

yq -e 'select(.kind == "Application" and .metadata.name == "b-rook-ceph-provisioning") |
  .spec.syncPolicy.automated.prune == null' "$TMP_DIR/b.yaml" >/dev/null
yq -e 'select(.kind == "Application" and .metadata.name == "b-rook-ceph-rgw") |
  .spec.syncPolicy.automated.prune == true' "$TMP_DIR/b.yaml" >/dev/null

yq -e 'select(.kind == "Application" and .metadata.name == "tower-karmada-members") |
  .spec.syncPolicy.syncOptions[] == "SkipDryRunOnMissingResource=true"' \
  "$TMP_DIR/tower.yaml" >/dev/null

if yq -e 'select(.kind == "Application" and
  .metadata.name == "tower-karmada-objectbucket-api")' \
  "$TMP_DIR/tower.yaml" >/dev/null 2>&1; then
  echo "Tower render unexpectedly contains the retired OBC API bridge" >&2
  exit 1
fi

for cluster in b c; do
  if yq -e "select(.kind == \"Application\" and .metadata.name == \"$cluster\")" \
    "$TMP_DIR/${cluster}-child-only.yaml" >/dev/null 2>&1; then
    echo "child-only render unexpectedly contains root Application/$cluster" >&2
    exit 1
  fi
  yq -e "select(.kind == \"Application\" and .metadata.name == \"$cluster-cilium\")" \
    "$TMP_DIR/${cluster}-child-only.yaml" >/dev/null
done

echo "application sync order regression: PASS"
