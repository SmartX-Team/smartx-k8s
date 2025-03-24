#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

###########################################################
#   Utilities                                             #
###########################################################

function find_manifest() {
    "${__BIN_HOME}/manifest-find.sh"
}

function get_manifest() {
    "${__BIN_HOME}/manifest-get.sh"
}

function usage() {
    echo "Usage: $0 [namespace] [name] [url] [version]" >&2
    exit 1
}

###########################################################
#   Main Function                                         #
###########################################################

function main() {
    export __BIN_HOME="$(realpath "$(dirname $0)")"
    if [ "$#" -ne 4 ]; then
        usage
    fi

    namespace="$1"
    name="$2"
    version="$3"
    repoUrl="$4"

    repo_home="$(pwd)/apps/${name}"
    if [ -d "${repo_home}" ]; then
        echo "Already exists: ${repo_home}" >&2
        usage
    fi
    mkdir "${repo_home}"

    # Copy the template
    template_home="$(pwd)/apps/template"
    cat "${template_home}/manifest.yaml" |
        yq ".metadata.name=\"smartx.apps.${name}\"" |
        yq ".spec.app.namespace=\"${namespace}\"" |
        yq ".spec.helm.chart=\"${name}\"" |
        yq ".spec.helm.repoUrl=\"${repoUrl}\"" |
        yq ".spec.helm.version=\"${version}\"" |
        cat >"${repo_home}/manifest.yaml"
    cp "${template_home}/values.yaml" "${repo_home}/values.yaml"
}

main $@
