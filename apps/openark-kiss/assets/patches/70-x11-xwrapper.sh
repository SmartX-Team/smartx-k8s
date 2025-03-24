#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Desktop Environment Configuration
# Enable user-level virtual console (xf86) acccess

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

cat <<EOF >/etc/X11/Xwrapper.config
allowed_users=console
needs_root_rights=yes
EOF
