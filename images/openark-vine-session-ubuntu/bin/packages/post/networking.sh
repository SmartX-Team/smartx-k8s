#!/usr/bin/env bash
# Copyright (c) 2023 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Backup SSH Configuration

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

rm -rf /etc/ssh/ssh_host_*
cp -r /etc/ssh /etc/.ssh
