#!/bin/sh
# SPDX-FileCopyrightText: 2023 sublipri <sublipri@proton.me>
# SPDX-License-Identifier: GPL-3.0-only

printenv | sort | logger -p 7 -t wifiremote-devel
if grep -q -e "^ForceWifiOn=true" <"$KOBO_CONFIG_FILE"; then
	sed -i -e "s/^ForceWifiOn=true/ForceWifiOn=false/" "$KOBO_CONFIG_FILE"
	message="Force Wi-Fi was disabled."
elif grep -q -e "^ForceWifiOn=false" <"$KOBO_CONFIG_FILE"; then
	sed -i -e "s/^ForceWifiOn=false/ForceWifiOn=true/" "$KOBO_CONFIG_FILE"
	message="Force Wi-Fi was enabled."
else
	# QSettings will merge this with any existing DeveloperSettings
	printf "\n[DeveloperSettings]\nForceWifiOn=true\n" >>"$KOBO_CONFIG_FILE"
	message="Force Wi-Fi was enabled."
fi
logger -p 6 -t wifiremote-devel "$message"

output-html "$message</p><p>Your Kobo will reboot to apply the changes."
reboot -d 1 &
