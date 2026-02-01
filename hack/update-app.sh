#!/usr/bin/env bash
# Copyright (c) 2025-2026 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Update all applications

# Prehibit errors
set -e -o pipefail

###########################################################
#   Main Function                                         #
###########################################################

# Update the helm repositories
helm repo update >/dev/null

# List all app manifests
for manifest_path in $(
    find "$(dirname $(dirname $0))/apps" \
        -maxdepth 2 -mindepth 2 -name 'manifest.yaml' | sort
); do
    # Skip manifest template
    if [ "x${manifest_path}" == 'x./apps/template/manifest.yaml' ]; then
        continue
    fi

    # Skip template apps
    if ! cat "${manifest_path}" | yq >/dev/null 2>/dev/null; then
        continue
    fi

    # Skip non-external sources
    if [ "x$(cat "${manifest_path}" | yq -r '.spec.source == null')" == 'xtrue' ]; then
        continue
    fi

    # Read the source chart name
    app_name="$(cat "${manifest_path}" | yq -r '.metadata.name')"
    if [ "x${app_name}" == 'xnull' ]; then
        echo "Cannot read app name: ${app_name}" >&2
        exit 1
    fi

    # Read the source chart name
    source_chart="$(cat "${manifest_path}" | yq -r '.spec.source.chart')"
    if [ "x${source_chart}" == 'xnull' ]; then
        echo "Cannot read app chart name: ${manifest_path}" >&2
        exit 1
    fi

    # Read the source repository URL
    source_repo_url="$(cat "${manifest_path}" | yq -r '.spec.source.repoUrl')"
    if [ "x${source_repo_url}" == 'xnull' ]; then
        echo "Cannot read app repository URL: ${manifest_path}" >&2
        exit 1
    fi

    # Mark OCI repository
    source_is_oci="$(echo "x${source_repo_url}" | grep -Posq '^xhttps.*$' || echo 'true')"

    # Read the source version
    source_version="$(cat "${manifest_path}" | yq -r '.spec.source.version')"
    if [ "x${source_version}" == 'xnull' ]; then
        echo "Cannot read app source version: ${manifest_path}" >&2
        exit 1
    fi

    # Load the latest chart version
    if [ "x${source_is_oci}" == 'xtrue' ]; then
        latest_version="$(helm show chart "oci://${source_repo_url}/${source_chart}" | yq -r '.version')" || continue
    else
        latest_version="$(helm show chart --repo "${source_repo_url}" "${source_chart}" | yq -r '.version')" || continue
    fi

    # Test the version
    if [ "x${source_version}" != "x${latest_version}" ]; then
        # Write a report
        echo -n '# '${manifest_path}'
---
apiVersion: org.ulagbulag.io/v1
kind: Report
metadata:
    name: "'"${app_name}"'"
spec:
    type: PullRequest
    version:
        source: >
            '"${source_version}"'
        target: >
            '"${latest_version}"'
'
    fi
done
