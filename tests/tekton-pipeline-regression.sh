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

helm template tower "$EECS_DIR" \
  -f "$TOWER_DIR/values.yaml" \
  >"$TMP_DIR/applications.yaml"

helm template tekton-pipeline tekton-pipeline \
  --repo https://cdfoundation.github.io/tekton-helm-chart/ \
  --version 1.14.0 \
  --namespace tekton-pipelines \
  --include-crds \
  -f "$EECS_DIR/apps/tekton-pipeline/values.yaml" \
  -f "$TOWER_DIR/patches/tekton-pipeline/values.yaml" \
  >"$TMP_DIR/tekton-pipeline.yaml"

helm template workload-namespace "$EECS_DIR/apps/workload-namespace" \
  -f "$EECS_DIR/apps/workload-namespace/values.yaml" \
  -f "$TOWER_DIR/patches/workload-namespace/values.yaml" \
  >"$TMP_DIR/workload-namespace.yaml"

python3 - "$TMP_DIR/applications.yaml" "$TMP_DIR/tekton-pipeline.yaml" "$TMP_DIR/workload-namespace.yaml" "$TOWER_DIR/values.yaml" <<'PY'
import sys
import yaml

applications_path, tekton_path, workload_namespace_path, tower_values_path = sys.argv[1:]

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
    and document.get("metadata", {}).get("name") == "tower-tekton-pipeline"
]
assert len(matches) == 1, f"expected one Tower Tekton Application, got {len(matches)}"
application = matches[0]
assert application["spec"]["destination"] == {
    "name": "tower",
    "namespace": "tekton-pipelines",
}

sources = application["spec"]["sources"]
assert sources[0]["repoURL"] == "https://cdfoundation.github.io/tekton-helm-chart/"
assert sources[0]["chart"] == "tekton-pipeline"
assert str(sources[0]["targetRevision"]) == "1.14.0"
assert sources[0]["helm"]["valueFiles"] == [
    "$origin/apps/tekton-pipeline/values.yaml",
    "$cluster/patches/tekton-pipeline/values.yaml",
]
assert sources[-1]["repoURL"] == "https://github.com/SmartX-Team/sandbox-tower-k8s.git"
assert sources[1]["targetRevision"] == origin_revision
assert sources[-1]["targetRevision"] == cluster_revision
assert sources[-1]["ref"] == "cluster"

workload_applications = [
    document
    for document in applications
    if document.get("kind") == "Application"
    and document.get("metadata", {}).get("name") == "tower-workload-namespace"
]
assert len(workload_applications) == 1

with open(tekton_path, encoding="utf-8") as stream:
    resources = [document for document in yaml.safe_load_all(stream) if document]

namespaces = [
    resource
    for resource in resources
    if resource.get("kind") == "Namespace"
]
assert len(namespaces) == 1
assert namespaces[0]["metadata"]["name"] == "tekton-pipelines"
assert namespaces[0]["metadata"]["labels"]["pod-security.kubernetes.io/enforce"] == "restricted"

for resource in resources:
    conversion = resource.get("spec", {}).get("conversion", {})
    service = conversion.get("webhook", {}).get("clientConfig", {}).get("service")
    if service:
        assert service["namespace"] == "tekton-pipelines"
    for webhook in resource.get("webhooks", []):
        service = webhook.get("clientConfig", {}).get("service")
        if service and service.get("name") == "tekton-pipelines-webhook":
            assert service["namespace"] == "tekton-pipelines"

required_deployments = {
    "tekton-events-controller",
    "tekton-pipelines-controller",
    "tekton-pipelines-remote-resolvers",
    "tekton-pipelines-webhook",
}
deployments = {
    resource["metadata"]["name"]: resource
    for resource in resources
    if resource.get("kind") == "Deployment"
}
assert required_deployments <= deployments.keys()
for name in required_deployments:
    containers = deployments[name]["spec"]["template"]["spec"]["containers"]
    assert containers, f"deployment {name} has no containers"
    assert "@sha256:" in containers[0]["image"], f"deployment {name} image is not digest-pinned"
    assert ":v1.14.0@" in containers[0]["image"], f"deployment {name} is not Tekton v1.14.0"

trigger_kinds = {
    "ClusterTriggerBinding",
    "EventListener",
    "Trigger",
    "TriggerBinding",
    "TriggerTemplate",
}
assert not [resource for resource in resources if resource.get("kind") in trigger_kinds]

with open(workload_namespace_path, encoding="utf-8") as stream:
    workload_resources = [document for document in yaml.safe_load_all(stream) if document]
assert len(workload_resources) == 1
workload_namespace = workload_resources[0]
assert workload_namespace["kind"] == "Namespace"
assert workload_namespace["metadata"]["name"] == "tower-ci"
assert workload_namespace["metadata"]["labels"]["pod-security.kubernetes.io/enforce"] == "restricted"
assert workload_namespace["metadata"]["labels"]["scalex.io/workload-type"] == "ci"
PY

echo "Tekton Pipeline regression: PASS"
