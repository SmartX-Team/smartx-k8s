# Copyright (c) 2025-2026 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Load a `.env` file, if present.
set dotenv-load

# Build and push an image within ./images
build-image NAME *ARGS:
    @clear
    @IMAGE_NAME={{ NAME }} ./hack/image-build.sh {{ ARGS }}

# Build and push an ISO with specific preset repository (local path or URL)
build-iso REPO:
    @clear
    @PRESET_URL={{ REPO }} ./hack/iso-build.sh

# Execute a command in the boxes
batch COMMAND *ARGS:
    @./hack/box-batch.sh {{ COMMAND }} {{ ARGS }}

# Bootstrap a new k8s cluster with specific preset repository (local path or URL)
bootstrap REPO:
    @PRESET_URL={{ REPO }} ./hack/kubespray.sh 'cluster.yml'

# List all boxes
box *ARGS:
    @./hack/box-ls.sh {{ ARGS }}

# Run development package: openark-cli
run *ARGS:
    @cargo run --package openark-cli -- \
        {{ ARGS }} \

# Run development package: dark-lake
run-dark-lake PROFILE *ARGS:
    @RUSTFLAGS="--cfg=io_uring_skip_arch_check" \
        cargo run --package dark-lake --profile "{{ PROFILE }}" -- {{ ARGS }} \

# Run development package: openark-spectrum-backend
run-openark-spectrum-backend:
    @cargo run --package openark-spectrum-backend -- \
        --default-record-service "$(cat ./apps/openark-spectrum-operator/values.yaml | yq -r '.prometheus.defaultRecords.service')" \
        --label-custom-histogram-record "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/spectrum-histogram-record"')" \
        --prometheus-base-url "$(cat ./apps/openark-spectrum-operator/values.yaml | yq -r '.prometheus.baseUrl')" \

# Run development package: openark-spectrum-operator
run-openark-spectrum-operator:
    @cargo run --package openark-spectrum-operator -- \
        --controller-name 'openark-spectrum-operator' \
        --install-crds \
        --label-histogram-parent "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/spectrum-histogram"')" \
        --label-histogram-weight "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/spectrum-histogram-weight"')" \
        --label-pool-claim-lifecycle-pre-start "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/spectrum-pool-claim-lifecycle-pre-start"')" \
        --label-pool-claim-parent "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/spectrum-pool-claim"')" \
        --label-pool-claim-priority "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/spectrum-pool-claim-priority"')" \
        --label-pool-claim-weight "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/spectrum-pool-claim-weight"')" \
        --label-pool-claim-weight-penalty "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/spectrum-pool-claim-weight-penalty"')" \
        --label-pool-claim-weight-max "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/spectrum-pool-claim-weight-max"')" \
        --label-pool-claim-weight-min "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/spectrum-pool-claim-weight-min"')" \
        --label-pool-parent "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/spectrum-pool"')" \
        --pool-base-url 'http://localhost:9000' \
        --upgrade-crds \

# Run development package: openark-spectrum-pool
run-openark-spectrum-pool:
    @cargo run --package openark-spectrum-pool -- \
        --base-url '' \
        --max-pool '64' \

# Run development package: openark-vine-browser-backend
run-openark-vine-browser-backend:
    @cargo run --package openark-vine-browser-backend -- \
        --base-url '/api/v1' \
        --oauth-client-origin="http://localhost:8080"

# Run development package: openark-vine-browser-frontend
run-openark-vine-browser-frontend:
    @cd ./crates/openark-vine-browser-frontend && \
        env API_BASE_URL="http://localhost:8888/api/v1/" \
        trunk serve \

# Run development package: openark-vine-session-backend
run-openark-vine-dashboard-backend:
    @cargo run --package openark-vine-dashboard-backend -- \
        --base-url '/api/v1' \
        --label-category "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/category"')" \
        --label-description "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/description"')" \
        --label-title "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/title"')" \

# Run development package: openark-vine-dashboard-frontend
run-openark-vine-dashboard-frontend:
    @cd ./crates/openark-vine-dashboard-frontend && \
        env OPENARK_LABEL_ALIAS="$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/alias"')" \
        trunk serve \

# Run development package: openark-vine-session-backend
run-openark-vine-session-backend:
    cargo run --package openark-vine-session-backend -- \
        --base-url '/api/v1' \
        --label-bind "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/bind"')" \
        --label-bind-user "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/bind.user"')" \

# Run development package: openark-vine-session-operator
run-openark-vine-session-operator:
    @cargo run --package openark-vine-session-operator -- \
        --controller-name 'openark-vine-session-operator' \
        --install-crds \
        --duration-sign-out-seconds "$(cat ./apps/openark-vine-session-operator/values.yaml | yq -r '.operator.signOutTimeoutSeconds')" \
        --label-alias "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/alias"')" \
        --label-bind "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/bind"')" \
        --label-bind-cpu "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/bind.cpu"')" \
        --label-bind-memory "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/bind.memory"')" \
        --label-bind-mode "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/bind.mode"')" \
        --label-bind-namespace "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/bind.namespace"')" \
        --label-bind-node "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/bind.node"')" \
        --label-bind-persistent "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/bind.persistent"')" \
        --label-bind-privileged "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/bind.privileged"')" \
        --label-bind-profile "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/bind.profile"')" \
        --label-bind-revision "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/bind.revision"')" \
        --label-bind-storage "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/bind.storage"')" \
        --label-bind-timestamp "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/bind.timestamp"')" \
        --label-bind-user "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/bind.user"')" \
        --label-compute-mode "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/compute-mode"')" \
        --label-gpu "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/gpu"')" \
        --label-is-private "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/is-private"')" \
        --label-signed-out "$(cat ./values.yaml | yq -r '.openark.labels."org.ulagbulag.io/signed-out"')" \
        --session-namespace 'vine-session' \
        --source-path 'apps/openark-vine-session' \
        --source-repo-revision "$(cat ./values.yaml | yq -r '.repo.revision')" \
        --source-repo-url "$(cat ./values.yaml | yq -r '.repo.baseUrl')/$(cat ./values.yaml | yq -r '.repo.owner')/$(cat ./values.yaml | yq -r '.repo.name').git" \
        --upgrade-crds \

# Execute a command in a box
ssh BOX *ARGS:
    @./hack/box-ssh.sh {{ BOX }} {{ ARGS }}

# Update all applications (Helm charts, etc.)
update:
    @./hack/update-app.sh

# Validate project
validate:
    ./hack/git-ci-validate.sh
    typos
