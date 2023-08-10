#!/bin/sh
# SPDX-FileCopyrightText: 2023 sublipri <sublipri@proton.me>
# SPDX-License-Identifier: GPL-3.0-only

valid_options="AutoUsbGadget ShowKeyboardTaps ForceAllowLandscape ForceWifiOn"
option="$QUERY_STRING"
if ! echo "$valid_options" | grep -q "$option"; then
	output-html "<strong>$option</strong> is not a valid option"
	exit
fi

printenv | sort | logger -p 7 -t wifiremote-devel
logger -p 7 -t wifiremote-devel -- "--- Old Developer Settings ---"
awk '/\[DeveloperSettings/,/^$/' "$KOBO_CONFIG_FILE" | logger -p 7 -t wifiremote-devel
if grep -q -e "^$option=true" <"$KOBO_CONFIG_FILE"; then
	sed -i -e "s/^$option=true/$option=false/" "$KOBO_CONFIG_FILE"
	message="<strong>$option</strong> was disabled."
elif grep -q -e "^$option=false" <"$KOBO_CONFIG_FILE"; then
	sed -i -e "s/^$option=false/$option=true/" "$KOBO_CONFIG_FILE"
	message="<strong>$option</strong> was enabled."
else
	# QSettings will merge this with any existing DeveloperSettings
	printf "\n[DeveloperSettings]\n%s=true\n" "$option" >>"$KOBO_CONFIG_FILE"
	message="<strong>$option</strong> was enabled."
fi
logger -p 6 -t wifiremote-devel "$message"

sleep 1 && logger -p 7 -t wifiremote-devel -- "--- New Developer Settings --- " &&
	awk '/\[DeveloperSettings/,/^$/' "$KOBO_CONFIG_FILE" | logger -p 7 -t wifiremote-devel &

output-html "$message</p><p>Reboot your Kobo to apply the changes."
