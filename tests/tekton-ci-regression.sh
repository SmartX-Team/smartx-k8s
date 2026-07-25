#!/usr/bin/env bash

set -euo pipefail

EECS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TOWER_DIR="$(dirname "$EECS_DIR")/tower-k8s"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

require() {
  command -v "$1" >/dev/null || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

for command in helm python3; do
  require "$command"
done

helm lint "$EECS_DIR/apps/tekton-ci" \
  -f "$EECS_DIR/apps/tekton-ci/values.yaml" \
  -f "$TOWER_DIR/patches/tekton-ci/values.yaml" >/dev/null

helm template tower "$EECS_DIR" \
  -f "$TOWER_DIR/values.yaml" \
  >"$TMP_DIR/applications.yaml"

helm template tekton-ci "$EECS_DIR/apps/tekton-ci" \
  --namespace tower-ci \
  -f "$EECS_DIR/apps/tekton-ci/values.yaml" \
  -f "$TOWER_DIR/patches/tekton-ci/values.yaml" \
  >"$TMP_DIR/tekton-ci.yaml"

python3 - "$TMP_DIR/applications.yaml" "$TMP_DIR/tekton-ci.yaml" "$TOWER_DIR/values.yaml" <<'PY'
import sys
import yaml

applications_path, resources_path, tower_values_path = sys.argv[1:]

with open(tower_values_path, encoding="utf-8") as stream:
    tower_values = yaml.safe_load(stream)
origin_revision = tower_values["repo"]["revision"]
cluster_revision = tower_values["cluster"]["group"]

with open(applications_path, encoding="utf-8") as stream:
    applications = [document for document in yaml.safe_load_all(stream) if document]

matches = [
    document
    for document in applications
    if document.get("kind") == "Application"
    and document.get("metadata", {}).get("name") == "tower-tekton-ci"
]
assert len(matches) == 1, f"expected one tower-tekton-ci Application, got {len(matches)}"
application = matches[0]
assert application["spec"]["destination"] == {
    "name": "tower",
    "namespace": "tower-ci",
}
assert application["metadata"]["annotations"]["argocd.argoproj.io/sync-wave"] == "20"
assert application["spec"]["syncPolicy"]["syncOptions"] == [
    "CreateNamespace=false",
    "RespectIgnoreDifferences=true",
    "ServerSideApply=true",
]
sources = application["spec"]["sources"]
assert sources[0]["repoURL"] == "https://github.com/SJoon99/eecs-k8s.git"
assert sources[0]["path"] == "apps/tekton-ci"
assert sources[0]["targetRevision"] == origin_revision
assert sources[0]["helm"]["releaseName"] == "tekton-ci"
assert sources[0]["helm"]["valueFiles"] == [
    "$cluster/patches/tekton-ci/values.yaml",
]
assert sources[1] == {
    "repoURL": "https://github.com/SmartX-Team/sandbox-tower-k8s.git",
    "targetRevision": cluster_revision,
    "ref": "cluster",
}

with open(resources_path, encoding="utf-8") as stream:
    resources = [document for document in yaml.safe_load_all(stream) if document]

expected = {
    ("ServiceAccount", "tekton-ci-runner"),
    ("Role", "tekton-ci-runner"),
    ("RoleBinding", "tekton-ci-runner"),
    ("ResourceQuota", "tower-ci"),
    ("LimitRange", "tower-ci"),
    ("NetworkPolicy", "tower-ci-deny-ingress"),
    ("PersistentVolumeClaim", "source-workspace"),
    ("Task", "child-validate-input"),
    ("Task", "child-clone-exact-sha"),
    ("Task", "child-helm-validate"),
    ("Task", "child-buildkit-build-push"),
    ("Pipeline", "child-build"),
}
actual = {(resource["kind"], resource["metadata"]["name"]) for resource in resources}
assert actual == expected, f"unexpected Tower CI resources: {actual ^ expected}"
assert all(resource["metadata"]["namespace"] == "tower-ci" for resource in resources)
assert not [resource for resource in resources if resource["kind"] == "Secret"]
assert not [resource for resource in resources if resource["kind"] in {"PipelineRun", "TaskRun"}]

by_kind = {resource["kind"]: resource for resource in resources}
service_account = by_kind["ServiceAccount"]
assert service_account["automountServiceAccountToken"] is False
assert "imagePullSecrets" not in service_account

role = by_kind["Role"]
assert role["rules"] == []
role_binding = by_kind["RoleBinding"]
assert role_binding["roleRef"] == {
    "apiGroup": "rbac.authorization.k8s.io",
    "kind": "Role",
    "name": "tekton-ci-runner",
}
assert role_binding["subjects"] == [
    {
        "kind": "ServiceAccount",
        "name": "tekton-ci-runner",
        "namespace": "tower-ci",
    }
]

quota = by_kind["ResourceQuota"]["spec"]["hard"]
assert quota == {
    "pods": "20",
    "requests.cpu": "4",
    "requests.memory": "8Gi",
    "limits.cpu": "8",
    "limits.memory": "16Gi",
    "persistentvolumeclaims": "10",
    "requests.storage": "100Gi",
}

limit = by_kind["LimitRange"]["spec"]["limits"]
assert limit == [
    {
        "type": "Container",
        "default": {"cpu": "2", "memory": "4Gi"},
        "defaultRequest": {"cpu": "100m", "memory": "128Mi"},
    }
]

network_policy = by_kind["NetworkPolicy"]["spec"]
assert network_policy == {
    "podSelector": {},
    "policyTypes": ["Ingress"],
}

pvc = by_kind["PersistentVolumeClaim"]["spec"]
assert pvc["storageClassName"] == "rook-ceph-filesystem-hot"
assert pvc["accessModes"] == ["ReadWriteMany"]
assert pvc["resources"]["requests"]["storage"] == "10Gi"

tasks = {
    resource["metadata"]["name"]: resource
    for resource in resources
    if resource["kind"] == "Task"
}
assert set(tasks) == {
    "child-validate-input",
    "child-clone-exact-sha",
    "child-helm-validate",
    "child-buildkit-build-push",
}
for task in tasks.values():
    for step in task["spec"]["steps"]:
        image = step["image"]
        assert "@sha256:" in image and not image.endswith(":latest"), image
        security = step.get("securityContext", task["spec"].get("stepTemplate", {}).get("securityContext"))
        assert security["runAsNonRoot"] is True
        assert security["allowPrivilegeEscalation"] is False
        assert security["capabilities"]["drop"] == ["ALL"]
        assert security["seccompProfile"]["type"] == "RuntimeDefault"

build_task = tasks["child-buildkit-build-push"]
secret_volume = build_task["spec"]["volumes"][0]
assert secret_volume["secret"]["secretName"] == "harbor-builder"
assert secret_volume["secret"]["items"] == [
    {"key": ".dockerconfigjson", "path": "config.json"}
]
build_env = {
    entry["name"]: entry["value"]
    for entry in build_task["spec"]["steps"][0]["env"]
}
assert build_env["REGISTRY_HOST"] == "10.34.25.18"
assert build_env["REGISTRY_REPOSITORY_PREFIX"] == "tower-ci"
assert build_env["REGISTRY_INSECURE"] == "true"

pipeline = by_kind["Pipeline"]
pipeline_tasks = pipeline["spec"]["tasks"]
assert [task["name"] for task in pipeline_tasks] == [
    "validate-input",
    "clone",
    "helm-validate",
    "build-push",
]
assert pipeline_tasks[1]["runAfter"] == ["validate-input"]
assert pipeline_tasks[2]["runAfter"] == ["clone"]
assert pipeline_tasks[3]["runAfter"] == ["helm-validate"]
assert {result["name"] for result in pipeline["spec"]["results"]} == {
    "source-revision",
    "image-url",
    "image-tag",
    "image-digest",
}
PY

echo "Tekton CI foundation and Child Pipeline regression: PASS"
