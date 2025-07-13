#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Desktop Environment Configuration
# DBus Configuration
# Grant all bluetooth permissions

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

mkdir -p /etc/dbus-1/system.d/
cat <<EOF >/etc/dbus-1/system.d/bluetooth.conf
<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-BUS Bus Configuration 1.0//EN" "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
    <!-- allow user to initialize and communicate -->
    <policy user="root">
        <allow own="org.bluez" />
        <allow send_destination="org.bluez" />
        <allow send_interface="org.bluez.AdvertisementMonitor1" />
        <allow send_interface="org.bluez.Agent1" />
        <allow send_interface="org.bluez.MediaEndpoint1" />
        <allow send_interface="org.bluez.MediaPlayer1" />
        <allow send_interface="org.bluez.Profile1" />
        <allow send_interface="org.bluez.GattCharacteristic1" />
        <allow send_interface="org.bluez.GattDescriptor1" />
        <allow send_interface="org.bluez.LEAdvertisement1" />
        <allow send_interface="org.freedesktop.DBus.ObjectManager" />
        <allow send_interface="org.freedesktop.DBus.Properties" />
        <allow send_interface="org.mpris.MediaPlayer2.Player" />
    </policy>
    <policy user="tenant">
        <allow own="org.bluez" />
        <allow send_destination="org.bluez" />
        <allow send_interface="org.bluez.AdvertisementMonitor1" />
        <allow send_interface="org.bluez.Agent1" />
        <allow send_interface="org.bluez.MediaEndpoint1" />
        <allow send_interface="org.bluez.MediaPlayer1" />
        <allow send_interface="org.bluez.Profile1" />
        <allow send_interface="org.bluez.GattCharacteristic1" />
        <allow send_interface="org.bluez.GattDescriptor1" />
        <allow send_interface="org.bluez.LEAdvertisement1" />
        <allow send_interface="org.freedesktop.DBus.ObjectManager" />
        <allow send_interface="org.freedesktop.DBus.Properties" />
        <allow send_interface="org.mpris.MediaPlayer2.Player" />
    </policy>
</busconfig>
EOF
