#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

# Detect architecture
OS_ARCH="$(uname -m)"
case "${OS_ARCH}" in
'aarch64')
    OS_ARCH='arm64'
    ;;

'x86_64')
    OS_ARCH='amd64'
    ;;
esac

# Install golang
curl {{ printf "%s/go%s.linux-${OS_ARCH}.tar.gz"
            .Values.build.golang.baseUrl
            .Values.build.golang.version
        | quote
    }} |
    tar -C /opt -zx

# Ready
exec true
