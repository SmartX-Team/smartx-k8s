#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

###########################################################
#   Main Function                                         #
###########################################################

# Configure destination file
DST_FILE='.github/CODEOWNERS'

# Clear destination file
printf '# AUTO-GENERATED; DO NOT EDIT\n\n' >"${DST_FILE}"

for path in $(find . -name manifest.yaml | sort -u); do
    # Skip template file
    if [ "${path}" == './apps/templates/manifest.yaml' ]; then
        continue
    fi

    # Infer path prefix
    prefix="$(dirname ${path})"

    # Use wildcard prefix
    if [ "${prefix}" == '.' ]; then
        prefix='.*'
    fi

    # Crawl owners
    owners="$(cat "${path}" | yq '.spec.users.owners[]' | grep -Po '(?<=<).*(?=>)' | paste -sd ' ')"

    echo "${prefix:1}" "${owners}" >>"${DST_FILE}"
done
