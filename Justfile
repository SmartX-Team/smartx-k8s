# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Load a `.env` file, if present.
set dotenv-load

# Build and push an image within ./images
build-image NAME:
    @clear
    @IMAGE_NAME={{ NAME }} ./hack/image-build.sh

# Execute a command in the boxes
batch COMMAND *ARGS:
    @./hack/box-batch.sh {{ COMMAND }} {{ ARGS }}

# Bootstrap a new k8s cluster
bootstrap REPO:
    @./hack/kubespray.sh {{ REPO }} 'cluster.yml'

# List all boxes
box *ARGS:
    @./hack/box-ls.sh {{ ARGS }}

# Run development package: openark-spectrum-backend
run-openark-spectrum-backend:
    @cargo run --package openark-spectrum-backend -- \
        --default-record-service "$(cat ./apps/openark-spectrum-operator/values.yaml | yq '.prometheus.defaultRecords.service')" \
        --label-custom-histogram-record "$(cat ./values.yaml | yq '.openark.labels."org.ulagbulag.io/spectrum-histogram-record"')" \
        --prometheus-base-url "$(cat ./apps/openark-spectrum-operator/values.yaml | yq '.prometheus.baseUrl')" \

# Run development package: openark-spectrum-operator
run-openark-spectrum-operator:
    @cargo run --package openark-spectrum-operator -- \
        --controller-name 'openark-histogram' \
        --install-crds \
        --label-histogram-parent "$(cat ./values.yaml | yq '.openark.labels."org.ulagbulag.io/spectrum-histogram"')" \
        --label-histogram-weight "$(cat ./values.yaml | yq '.openark.labels."org.ulagbulag.io/spectrum-histogram-weight"')" \
        --label-pool-claim-lifecycle-pre-start "$(cat ./values.yaml | yq '.openark.labels."org.ulagbulag.io/spectrum-pool-claim-lifecycle-pre-start"')" \
        --label-pool-claim-parent "$(cat ./values.yaml | yq '.openark.labels."org.ulagbulag.io/spectrum-pool-claim"')" \
        --label-pool-claim-priority "$(cat ./values.yaml | yq '.openark.labels."org.ulagbulag.io/spectrum-pool-claim-priority"')" \
        --label-pool-claim-weight "$(cat ./values.yaml | yq '.openark.labels."org.ulagbulag.io/spectrum-pool-claim-weight"')" \
        --label-pool-claim-weight-penalty "$(cat ./values.yaml | yq '.openark.labels."org.ulagbulag.io/spectrum-pool-claim-weight-penalty"')" \
        --label-pool-claim-weight-max "$(cat ./values.yaml | yq '.openark.labels."org.ulagbulag.io/spectrum-pool-claim-weight-max"')" \
        --label-pool-claim-weight-min "$(cat ./values.yaml | yq '.openark.labels."org.ulagbulag.io/spectrum-pool-claim-weight-min"')" \
        --label-pool-parent "$(cat ./values.yaml | yq '.openark.labels."org.ulagbulag.io/spectrum-pool"')" \
        --pool-base-url 'http://localhost:9000' \
        --upgrade-crds \

# Run development package: openark-spectrum-pool
run-openark-spectrum-pool:
    @cargo run --package openark-spectrum-pool -- \
        --base-url '' \
        --max-pool '64' \

# Run development package: openark-vine-session-backend
run-openark-vine-dashboard-backend:
    @cargo run --package openark-vine-dashboard-backend -- \
        --base-url '/api/v1' \
        --label-category "$(cat ./values.yaml | yq '.openark.labels."org.ulagbulag.io/category"')" \
        --label-description "$(cat ./values.yaml | yq '.openark.labels."org.ulagbulag.io/description"')" \
        --label-title "$(cat ./values.yaml | yq '.openark.labels."org.ulagbulag.io/title"')" \

# Run development package: openark-vine-dashboard-frontend
run-openark-vine-dashboard-frontend:
    @cd ./crates/openark-vine-dashboard-frontend && \
        env OPENARK_LABEL_ALIAS="$(cat ./values.yaml | yq '.openark.labels."org.ulagbulag.io/alias"')" \
        trunk serve \

# Run development package: openark-vine-session-backend
run-openark-vine-session-backend:
    cargo run --package openark-vine-session-backend -- \
        --base-url '/api/v1' \
        --label-bind "$(cat ./values.yaml | yq '.openark.labels."org.ulagbulag.io/bind"')" \
        --label-bind-user "$(cat ./values.yaml | yq '.openark.labels."org.ulagbulag.io/bind.user"')" \

# Execute a command in a box
ssh BOX *ARGS:
    @./hack/box-ssh.sh {{ BOX }} {{ ARGS }}

# Update all
update:
    @./hack/update-app.sh

# Validate project
validate:
    ./hack/git-ci-validate.sh
    typos
