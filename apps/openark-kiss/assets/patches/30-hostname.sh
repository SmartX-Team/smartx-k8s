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
HOSTS_TMP="$(mktemp)"
trap 'rm -f "${HOSTS_TMP}"' EXIT

awk -v uuid="${UUID}" '
$1 == "127.0.0.1" {
    for (i = 2; i <= NF; i++) {
        if ($i ~ /^#/) {
            break
        }

        if ($i == "localhost" || tolower($i) == tolower(uuid)) {
            continue
        }

        if (!seen_alias[$i]++) {
            aliases[++alias_count] = $i
        }
    }

    next
}

{
    lines[++line_count] = $0
}

END {
    printf "127.0.0.1 localhost %s", uuid

    for (i = 1; i <= alias_count; i++) {
        printf " %s", aliases[i]
    }

    printf "\n"

    for (i = 1; i <= line_count; i++) {
        print lines[i]
    }
}
' /etc/hosts >"${HOSTS_TMP}"

cat "${HOSTS_TMP}" >/etc/hosts
printf '%s\n' "${UUID}" >/etc/hostname

getent ahostsv4 localhost | grep -q '^127\.0\.0\.1[[:space:]]'
