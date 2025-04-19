#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

# Wait until the ollama is ready
until ollama ps >/dev/null 2>/dev/null; do
    sleep 1
done

# Pull image
until ollama pull "${OPENAI_MODEL_NAME}"; do
    sleep 1
done

# Ready
exec true
