#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Data Pond Storage
# Load NVMe Kernel Modules

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# nvme
modprobe nvme num_p2p_queues=1
modprobe nvme-tcp

# nvmet
modprobe nvmet
modprobe nvmet-tcp
