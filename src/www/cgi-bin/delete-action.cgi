#!/bin/sh
# SPDX-FileCopyrightText: 2023 sublipri <sublipri@proton.me>
# SPDX-License-Identifier: GPL-3.0-only

printenv | sort | logger -p 7 -t wifiremote-delete
QUERY_STRING=$(echo "$QUERY_STRING" | sed -e "s/\*/%2A/g" -e "s/\./%2E/g" -e "s/+/%20/g")
get_var() {
	echo "$QUERY_STRING" | sed -n "s|^.*${1}=\([^&]*\).*$|\1|p"
}

logger -p 7 -t wifiremote-delete "Encoded Query String: $QUERY_STRING"
path_segment=$(get_var "path-segment")
if [ "$path_segment" = "" ]; then
	message="No action selected."
elif grep -q "^$path_segment," <"$CSV_FILE"; then
	name=$(grep "^$path_segment," <"$CSV_FILE" | cut -d, -f2)
	name=$(httpd -d "$name")
	if [ -x "$EVENTS_DIR/$path_segment.evemu" ]; then
		rm -v "$EVENTS_DIR/$path_segment.evemu" 2>&1 | logger -p 6 -t wifiremote-delete
	fi
	if [ -x "$EVENTS_DIR/$path_segment.dat" ]; then
		rm -v "$EVENTS_DIR/$path_segment.dat" 2>&1 | logger -p 6 -t wifiremote-delete
	fi
	rmdir -v "$HTTP_DIR/$path_segment" | logger -p 6 -t wifiremote-delete
	logger -p 7 -t wifiremote-record "Original CSV Contents:"
	logger -p 7 -t wifiremote-record <"$CSV_FILE"
	sed -i -e "/^$path_segment,/d" "$CSV_FILE"
	logger -p 7 -t wifiremote-record "Updated CSV Contents:"
	logger -p 7 -t wifiremote-record <"$CSV_FILE"
	message="<strong>$name</strong> action deleted."
else
	message="<strong>$path_segment</strong> action does not exist."
fi

output-html "$message"
