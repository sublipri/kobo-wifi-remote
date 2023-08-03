#!/bin/sh
# SPDX-FileCopyrightText: 2023 sublipri <sublipri@proton.me>
# SPDX-License-Identifier: GPL-3.0-only

debug_enabled=false
force_wifi_enabled=false

if grep -q -e "\[DeveloperSettings\]" <"$KOBO_CONFIG_FILE"; then
	grep -q -e "^EnableDebugServices=true" <"$KOBO_CONFIG_FILE" && debug_enabled=true
	grep -q -e "^ForceWifiOn=true" <"$KOBO_CONFIG_FILE" && force_wifi_enabled=true
fi

if "$force_wifi_enabled"; then
	action="Disable"
else
	action="Enable"
fi

html="<a href=\"/cgi-bin/toggle-wifi.cgi\"><button>$action Force Wi-Fi On</button></a>"

if "$debug_enabled"; then
	html="${html}</p><p><strong>Warning:</strong> Debug services are enabled. 
	This allows anyone on the same network as your Kobo full remote access. 
	If this is not intentional, disable them now.
	You might have done this without understanding the implications by following advice
	online to access the Developer Settings by searching for 'devmodeon'.</p>
	<p><a href=\"/cgi-bin/disable-debug.cgi\"><button>Disable Debug Services</button></a>"
fi

output-html "$html"
