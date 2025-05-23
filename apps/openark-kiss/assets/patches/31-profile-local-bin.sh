#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Environment Variables Configuration

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

mkdir -p /etc/profile.d/

cat <<EOF >/etc/profile.d/path-local-bin.sh
# local binary path registration

export PATH=\${PATH}:/usr/local/bin
export PATH=\${PATH}:/opt/bin
EOF
