#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

# Wait until the kafka cluster is ready
until /opt/kafka/bin/kafka-cluster.sh cluster-id --bootstrap-server "${KAFKA_BOOTSTRAP_SERVERS}:9092"; do
    sleep 1
done

# Ready
exec true
