#!/bin/sh
# SPDX-FileCopyrightText: 2023 sublipri <sublipri@proton.me>
# SPDX-License-Identifier: GPL-3.0-only

printenv | sort | logger -p 7 -t wifiremote-devel
options="ForceWifiOn AutoUsbGadget ShowKeyboardTaps ForceAllowLandscape"

html=""
for option in $options; do
	if grep -q -e "^$option=" <"$KOBO_CONFIG_FILE"; then
		enabled=$(sed -n -e "s|^$option=\(\(true\|false\)\)\$|\1|p" <"$KOBO_CONFIG_FILE")
		logger -p 7 -t wifiremote-devel "$option=$enabled"
	else
		enabled=false
		logger -p 7 -t wifiremote-devel "$option is unset"
	fi
	if "$enabled"; then
		action="Disable"
	else
		action="Enable"
	fi
	html="${html}<a href=\"/cgi-bin/toggle-setting.cgi?$option\"><button>$action $option</button></a></p><p>"
done

html="${html}<a href='https://wiki.mobileread.com/wiki/Kobo_Configuration_Options#.5BDeveloperSettings.5D'>See here</a> for a description of options.</p>"

if grep -q -e "^EnableDebugServices=true" <"$KOBO_CONFIG_FILE"; then
	logger -p 7 -t wifiremote-devel "DebugServices are enabled"
	html="${html}<p><strong>Warning:</strong> Debug services are enabled. 
	This allows anyone on the same network as your Kobo full remote access. 
	If this is not intentional, disable them now.
	You might have done this without understanding the implications by following advice
	online to access the Developer Settings by searching for 'devmodeon'.</p>
	<p><a href=\"/cgi-bin/disable-debug.cgi\"><button>Disable Debug Services</button></a>"
fi

output-html "$html"
