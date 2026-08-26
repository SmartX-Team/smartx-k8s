#!/usr/bin/env bash

set -euo pipefail

export LC_ALL=C
export LANG=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"

cargo test \
    --manifest-path "${ROOT}/Cargo.toml" \
    --package openark-rook-ceph-controller

"${ROOT}/apps/rook-ceph-provisioning/tests/render.sh"
