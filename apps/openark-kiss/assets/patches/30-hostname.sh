#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Hostname Configuration

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

UUID="$(cat /sys/class/dmi/id/product_uuid)"
echo "127.0.0.1 ${UUID}" >>/etc/hosts
echo -n "${UUID}" >/etc/hostname
