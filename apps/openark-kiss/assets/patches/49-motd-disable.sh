#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# APT Packages Configuration
# Disable SSH Greeting Message

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

sed -i '/^[^#]*\<pam_motd.so\>/s/^/#/' /etc/pam.d/sshd
exec rm -rf /etc/update-motd.d
