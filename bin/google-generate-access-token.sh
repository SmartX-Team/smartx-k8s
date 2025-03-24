#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

###########################################################
#   Generate Access Token                                 #
###########################################################

exec echo -n "$("$(dirname "$0")/google-login.sh")" | jq -r '.access_token'
