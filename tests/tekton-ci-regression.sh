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
import re
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
    ("Service", "tekton-events-public"),
    ("NetworkPolicy", "tekton-events-public-ingress"),
    ("PersistentVolumeClaim", "source-workspace"),
    ("Task", "child-validate-input"),
    ("Task", "child-clone-exact-sha"),
    ("Task", "child-derive-build-targets"),
    ("Task", "child-helm-validate"),
    ("Task", "child-buildkit-build-push"),
    ("Task", "child-create-promotion-payload"),
    ("Task", "federation-promote"),
    ("Pipeline", "child-build"),
    ("Pipeline", "federation-promote"),
}
actual = {(resource["kind"], resource["metadata"]["name"]) for resource in resources}
assert actual == expected, f"unexpected Tower CI resources: {actual ^ expected}"
assert all(resource["metadata"]["namespace"] == "tower-ci" for resource in resources)
assert not [resource for resource in resources if resource["kind"] == "Secret"]
assert not [resource for resource in resources if resource["kind"] in {"PipelineRun", "TaskRun"}]
assert not [
    resource
    for resource in resources
    if resource["kind"] in {"EventListener", "TriggerBinding", "TriggerTemplate"}
]

by_identity = {
    (resource["kind"], resource["metadata"]["name"]): resource
    for resource in resources
}
service_account = by_identity[("ServiceAccount", "tekton-ci-runner")]
assert service_account["automountServiceAccountToken"] is False
assert "imagePullSecrets" not in service_account

role = by_identity[("Role", "tekton-ci-runner")]
assert role["rules"] == []
role_binding = by_identity[("RoleBinding", "tekton-ci-runner")]
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

quota = by_identity[("ResourceQuota", "tower-ci")]["spec"]["hard"]
assert quota == {
    "pods": "20",
    "requests.cpu": "4",
    "requests.memory": "8Gi",
    "limits.cpu": "8",
    "limits.memory": "16Gi",
    "persistentvolumeclaims": "10",
    "requests.storage": "100Gi",
}

limit = by_identity[("LimitRange", "tower-ci")]["spec"]["limits"]
assert limit == [
    {
        "type": "Container",
        "default": {"cpu": "2", "memory": "4Gi"},
        "defaultRequest": {"cpu": "100m", "memory": "128Mi"},
    }
]

network_policy = by_identity[("NetworkPolicy", "tower-ci-deny-ingress")]["spec"]
assert network_policy == {
    "podSelector": {},
    "policyTypes": ["Ingress"],
}

receiver_service = by_identity[("Service", "tekton-events-public")]["spec"]
assert receiver_service == {
    "type": "ClusterIP",
    "selector": {
        "app.kubernetes.io/managed-by": "EventListener",
        "app.kubernetes.io/part-of": "Triggers",
        "eventlistener": "repository-events",
    },
    "ports": [
        {
            "name": "http-listener",
            "protocol": "TCP",
            "port": 8080,
            "targetPort": 8080,
        }
    ],
}

receiver_policy = by_identity[("NetworkPolicy", "tekton-events-public-ingress")]["spec"]
assert receiver_policy == {
    "podSelector": {
        "matchLabels": {
            "app.kubernetes.io/managed-by": "EventListener",
            "app.kubernetes.io/part-of": "Triggers",
            "eventlistener": "repository-events",
        }
    },
    "policyTypes": ["Ingress"],
    "ingress": [{"ports": [{"protocol": "TCP", "port": 8080}]}],
}

pvc = by_identity[("PersistentVolumeClaim", "source-workspace")]["spec"]
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
    "child-derive-build-targets",
    "child-helm-validate",
    "child-buildkit-build-push",
    "child-create-promotion-payload",
    "federation-promote",
}
# The Child chart contract is enforced in CI: helm-validate renders and then
# checks name-independent invariants (workload/policy pairing, dangling
# selectors, release namespace, forbidden kinds). The validator is injected
# from files/ so a single copy serves every caller.
validate_task = tasks["child-helm-validate"]
assert [step["name"] for step in validate_task["spec"]["steps"]] == [
    "lint-and-render",
    "validate-contract",
]
assert {param["name"] for param in validate_task["spec"]["params"]} == {
    "child-name",
    "chart-path",
    "allowed-kinds",
}
allowed_kinds_param = next(
    param for param in validate_task["spec"]["params"] if param["name"] == "allowed-kinds"
)
assert allowed_kinds_param["default"] == ""
assert validate_task["spec"]["volumes"] == [{"name": "state", "emptyDir": {}}]
assert validate_task["spec"]["stepTemplate"]["volumeMounts"] == [
    {"name": "state", "mountPath": "/workspace-state"}
]
validate_scripts = {step["name"]: step["script"] for step in validate_task["spec"]["steps"]}
# /tmp is per-container, so the rendered manifest has to cross on the volume.
assert "/tmp/rendered.yaml" not in validate_scripts["lint-and-render"]
assert "/workspace-state/rendered.yaml" in validate_scripts["lint-and-render"]
assert 'helm template "$CHILD_NAME" "$chart" --namespace "$CHILD_NAME"' in validate_scripts["lint-and-render"]
contract_script = validate_scripts["validate-contract"]
assert "validate-render: profile=" in contract_script, "validator body was not injected"
assert "--profile=child" in contract_script
assert '--allow-kinds="$ALLOWED_KINDS"' in contract_script
for invariant in ("V1 ", "V2 ", "V4 ", "V5 "):
    assert invariant in contract_script, invariant
clone_script = tasks["child-clone-exact-sha"]["spec"]["steps"][0]["script"]
assert "git config --global" not in clone_script
assert clone_script.count('git -c safe.directory="$SOURCE_PATH"') == 4
payload_script = tasks["child-create-promotion-payload"]["spec"]["steps"][0]["script"]
assert "schemaVersion" in payload_script and "sourceRevision" in payload_script
assert '"images":load' in payload_script and "sha256" in payload_script
assert 'immutableTag == ("sha-" + strenv(SOURCE_REVISION))' in payload_script
assert 'test("^v(0|[1-9][0-9]*)' in payload_script
assert "all(.[];" not in payload_script
assert ")] | all" in payload_script
for task in tasks.values():
    for result in task["spec"].get("results", []):
        assert result["type"] == "string"
    for step in task["spec"]["steps"]:
        image = step["image"]
        assert re.fullmatch(r"[^@]+@sha256:[0-9a-f]{64}", image), image
        if task["metadata"]["name"] == "federation-promote":
            assert step["computeResources"] == {
                "requests": {"cpu": "10m", "memory": "32Mi"},
                "limits": {"cpu": "500m", "memory": "512Mi"},
            }
        elif task["metadata"]["name"] == "child-buildkit-build-push":
            # build-and-push (buildkit) is the only heavy step; the yq/crane
            # helpers are tiny. Tekton sums step limits, so the whole pod stays
            # well under the tower-ci ResourceQuota instead of consuming all of it.
            if step["name"] == "build-and-push":
                assert step["computeResources"] == {
                    "requests": {"cpu": "500m", "memory": "1Gi"},
                    "limits": {"cpu": "2", "memory": "4Gi"},
                }
            else:
                assert step["computeResources"] == {
                    "requests": {"cpu": "50m", "memory": "64Mi"},
                    "limits": {"cpu": "250m", "memory": "256Mi"},
                }
        else:
            assert step["computeResources"] == {}
        security = step.get("securityContext", task["spec"].get("stepTemplate", {}).get("securityContext"))
        assert security["runAsNonRoot"] is True
        if task["metadata"]["name"] == "child-buildkit-build-push" and step["name"] == "build-and-push":
            assert security["privileged"] is True
            assert security["allowPrivilegeEscalation"] is True
            assert security["seccompProfile"]["type"] == "Unconfined"
            assert security["appArmorProfile"]["type"] == "Unconfined"
        else:
            assert security.get("privileged", False) is False
            assert security["allowPrivilegeEscalation"] is False
            assert security["capabilities"]["drop"] == ["ALL"]
            assert security["seccompProfile"]["type"] == "RuntimeDefault"

build_task = tasks["child-buildkit-build-push"]
assert [param["name"] for param in build_task["spec"]["params"]] == [
    "child-name",
    "source-revision",
    "chart-path",
    "build-targets",
]
assert [step["name"] for step in build_task["spec"]["steps"]] == [
    "validate-targets",
    "build-and-push",
    "alias-semantic-tags",
    "emit-images",
]
validate_targets_script = next(
    step for step in build_task["spec"]["steps"] if step["name"] == "validate-targets"
)["script"]
assert '.images[strenv(IMAGE_NAME)].tag' in validate_targets_script
assert "child image tag must be an exact v-prefixed SemVer" in validate_targets_script
assert "valuesKey must equal image name" in validate_targets_script
build_push_script = next(
    step for step in build_task["spec"]["steps"] if step["name"] == "build-and-push"
)["script"]
assert 'immutable_tag="sha-${SOURCE_REVISION}"' in build_push_script
assert "validated-targets.tsv" in build_push_script
alias_script = next(
    step for step in build_task["spec"]["steps"] if step["name"] == "alias-semantic-tags"
)["script"]
assert "/ko-app/crane" in alias_script
assert "semantic tag already points to another artifact" in alias_script
assert "semantic tag alias verification failed" in alias_script
assert "crane tag" in alias_script
emit_images_script = next(step for step in build_task["spec"]["steps"] if step["name"] == "emit-images")["script"]
assert '.[strenv(KEY)]' in emit_images_script
assert '. [strenv(KEY)]' not in emit_images_script
assert '"tag": strenv(TAG)' in emit_images_script
assert '"immutableTag": strenv(IMMUTABLE_TAG)' in emit_images_script
secret_volume = build_task["spec"]["volumes"][1]
assert secret_volume["secret"]["secretName"] == "harbor-builder"
assert secret_volume["secret"]["items"] == [
    {"key": ".dockerconfigjson", "path": "config.json"}
]
build_env = {
    entry["name"]: entry["value"]
    for entry in build_task["spec"]["steps"][1]["env"]
}
assert build_env["REGISTRY_HOST"] == "10.34.25.18"
assert build_env["REGISTRY_REPOSITORY_PREFIX"] == "tower-ci"
assert build_env["REGISTRY_INSECURE"] == "true"

promotion_task = tasks["federation-promote"]
secret_refs = [
    env["valueFrom"]["secretKeyRef"]
    for step in promotion_task["spec"]["steps"]
    for env in step.get("env", [])
    if "valueFrom" in env
]
assert secret_refs == [
    {"name": "federation-promotion-github-app", "key": "appID"},
    {"name": "federation-promotion-github-app", "key": "privateKey"},
    {"name": "federation-promotion-github-app", "key": "installationID"},
]
promotion_script = "\n".join(step.get("script", "") for step in promotion_task["spec"]["steps"])
assert "openssl dgst -sha256 -sign" in promotion_script
assert "/app/installations/${GITHUB_INSTALLATION_ID}/access_tokens" in promotion_script
assert "rm -f /workspace/github-app-private-key.pem" in promotion_script
assert 'git ls-remote --heads origin "refs/heads/${branch}"' in promotion_script
assert '--force-with-lease="refs/heads/${branch}:${remote_revision}"' in promotion_script
assert '--force-with-lease="refs/heads/${branch}:"' in promotion_script
assert "git push --force " not in promotion_script
assert "https://api.github.com/repos/${GITHUB_REPOSITORY}/pulls" in promotion_script
assert "Problems parsing JSON" not in promotion_script
assert "'{\\\"title" not in promotion_script
assert "'{\"title\":\"%s\",\"head\":\"%s\"" in promotion_script
assert 'if type == "!!seq"' not in promotion_script
assert '[.] | flatten | .[0].html_url // ""' in promotion_script
assert "merge-base --is-ancestor" in promotion_script
assert "helm template" in promotion_script
# The payload defines the image set. The set may change (new or removed image)
# and a first enrollment needs no hand-seeded values, so the old equality check
# against the previously promoted Federation values is gone. Atomicity is instead
# enforced against the child chart inventory, and the change is surfaced in the
# pull request body.
assert "payload must replace the complete enrolled image set" not in promotion_script
assert "(.images // {}) | keys | sort" in promotion_script
assert "[.images[].name] | sort" in promotion_script
assert "initial image set: %s" in promotion_script
assert "image set changed: %s -> %s" in promotion_script
assert "payload does not match the child chart image inventory" in promotion_script
promotion_step_names = [step["name"] for step in promotion_task["spec"]["steps"]]
assert "verify-image-inventory" in promotion_step_names
assert promotion_step_names.index("verify-image-inventory") > promotion_step_names.index("verify-candidate")
assert promotion_step_names.index("verify-image-inventory") < promotion_step_names.index("render-candidate")
assert "${image_set_change}" in promotion_script
# A first enrollment creates an untracked releases/<child>/values.yaml, which
# `git diff` cannot see. Guard the worktree with `git status` instead.
assert "git status --porcelain --untracked-files=all" in promotion_script
assert "git diff --quiet" not in promotion_script
assert 'immutableTag == ("sha-" + strenv(SOURCE_REVISION))' in promotion_script
assert '.pullPolicy = "IfNotPresent"' in promotion_script
assert "all(.images[];" not in promotion_script
assert ")] | all)" in promotion_script
assert "main" not in [arg for arg in promotion_script.split() if arg.startswith("HEAD:refs/heads/main")]

pipelines = {
    resource["metadata"]["name"]: resource
    for resource in resources
    if resource["kind"] == "Pipeline"
}
pipeline = pipelines["child-build"]
assert "allowed-kinds" in {param["name"] for param in pipeline["spec"]["params"]}
helm_validate_task = next(
    task for task in pipeline["spec"]["tasks"] if task["name"] == "helm-validate"
)
assert {param["name"] for param in helm_validate_task["params"]} == {
    "child-name",
    "chart-path",
    "allowed-kinds",
}
pipeline_tasks = pipeline["spec"]["tasks"]
# Tower renders with promotion.enabled=true, so the build Pipeline also carries
# the promote Task that turns a successful build into a Federation pull request.
assert [task["name"] for task in pipeline_tasks] == [
    "validate-input",
    "clone",
    "derive-targets",
    "helm-validate",
    "build-push",
    "create-promotion-payload",
    "promote",
]
assert all(task["taskRef"]["kind"] == "Task" for task in pipeline_tasks)
pipeline_by_name = {task["name"]: task for task in pipeline_tasks}
assert pipeline_by_name["clone"]["runAfter"] == ["validate-input"]
assert pipeline_by_name["derive-targets"]["runAfter"] == ["clone"]
assert pipeline_by_name["helm-validate"]["runAfter"] == ["clone"]
# build-push waits for both the render gate and the resolved target set.
assert pipeline_by_name["build-push"]["runAfter"] == ["helm-validate", "derive-targets"]
assert pipeline_by_name["create-promotion-payload"]["runAfter"] == ["build-push"]
assert pipeline_by_name["promote"]["runAfter"] == ["create-promotion-payload"]

# build-targets is optional at the Pipeline level and resolved by derive-targets.
build_targets_param = next(
    param for param in pipeline["spec"]["params"] if param["name"] == "build-targets"
)
assert build_targets_param.get("default") == ""
derive_task = pipeline_by_name["derive-targets"]
assert derive_task["taskRef"] == {"kind": "Task", "name": "child-derive-build-targets"}
derive_params = {param["name"]: param["value"] for param in derive_task["params"]}
assert derive_params["build-targets"] == "$(params.build-targets)"
build_pipeline_params = {
    param["name"]: param["value"] for param in pipeline_by_name["build-push"]["params"]
}
assert build_pipeline_params["chart-path"] == "$(params.chart-path)"
# The build Task consumes the resolved set, not the raw Pipeline param.
assert build_pipeline_params["build-targets"] == "$(tasks.derive-targets.results.build-targets)"

# The in-Pipeline promote hop feeds the create-promotion-payload result to the
# same federation-promote Task the standalone promotion Pipeline uses. Merge stays
# human: this only opens the pull request.
promote_task = pipeline_by_name["promote"]
assert promote_task["taskRef"] == {"kind": "Task", "name": "federation-promote"}
promote_params = {param["name"]: param["value"] for param in promote_task["params"]}
assert promote_params["payload"] == "$(tasks.create-promotion-payload.results.payload)"

# The derive-targets Task derives from images/<name>/Dockerfile and passes an
# explicit array through unchanged.
derive_definition = tasks["child-derive-build-targets"]
assert {param["name"] for param in derive_definition["spec"]["params"]} == {"build-targets"}
derive_build_param = derive_definition["spec"]["params"][0]
assert derive_build_param["default"] == ""
derive_script = derive_definition["spec"]["steps"][0]["script"]
assert "images/%s/Dockerfile" in derive_script
assert "build-targets must be a JSON array" in derive_script
assert "no images/*/Dockerfile found and no explicit build-targets provided" in derive_script
assert '"context":"."' in derive_script
assert {result["name"] for result in pipeline["spec"]["results"]} == {
    "source-revision",
    "images",
    "promotion-payload",
}

promotion_pipeline = pipelines["federation-promote"]
assert [task["name"] for task in promotion_pipeline["spec"]["tasks"]] == ["promote"]
assert promotion_pipeline["spec"]["tasks"][0]["taskRef"] == {
    "kind": "Task",
    "name": "federation-promote",
}
assert {result["name"] for result in promotion_pipeline["spec"]["results"]} == {
    "branch",
    "changed",
    "pull-request-url",
}
PY

echo "Tekton CI foundation and Child Pipeline regression: PASS"
