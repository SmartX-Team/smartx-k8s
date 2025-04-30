#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# APT Packages Configuration
# Disable Unneeded Services

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# Default Services
cd /etc/systemd/system/
rm -f \
    dbus-org.freedesktop.ModemManager1.service \
    dbus-org.freedesktop.thermald.service \
    iscsi.service \
    syslog.service \
    vmtoolsd.service

# Groups
cd /etc/systemd/system/
rm -rf \
    display-manager.service.wants \
    graphical.target.wants \
    ModemManager \
    oem-config.service.wants \
    open-vm-tools.service.requires \
    remote-fs.target.wants

# Cloud-init
cd /etc/systemd/system/cloud-final.service.wants/
rm -f \
    snapd.seeded.service

# Final
cd /etc/systemd/system/final.target.wants/
rm -f \
    snapd.system-shutdown.service

# Multi-User
cd /etc/systemd/system/multi-user.target.wants/
rm -f \
    apport.service \
    cron.service \
    dmesg.service \
    lxd-installer.socket \
    ModemManager.service \
    nfs-client.target \
    open-vm-tools.service \
    rpcbind.service \
    rsyslog.service \
    snapd.apparmor.service \
    snapd.autoimport.service \
    snapd.core-fixup.service \
    snapd.recovery-chooser-trigger.service \
    snapd.seeded.service \
    snapd.service \
    thermald.service \
    ua-reboot-cmds.service \
    ubuntu-advantage.service \
    ufw.service \
    unattended-upgrades.service

# Paths
cd /etc/systemd/system/paths.target.wants/
rm -f \
    apport-autoreport.path

# Sockets
cd /etc/systemd/system/sockets.target.wants/
rm -f \
    apport-forward.socket \
    iscsid.socket \
    multipathd.socket \
    rpcbind.socket \
    snapd.socket

# SYSINIT
cd /etc/systemd/system/sysinit.target.wants/
rm -f \
    multipathd.service \
    open-iscsi.service

# Timers
cd /etc/systemd/system/timers.target.wants/
rm -f \
    apport-autoreport.timer \
    apt-daily.timer \
    apt-daily-upgrade.timer \
    fwupd-refresh.timer \
    motd-news.timer \
    snapd.snap-repair.timer \
    ua-timer.timer \
    update-notifier-download.timer \
    update-notifier-motd.timer
